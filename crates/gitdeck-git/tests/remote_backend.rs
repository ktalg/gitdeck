use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gitdeck_git::{CliGit, GitBackend, GitError, PullStrategy, PushRequest};
use gitdeck_testkit::{copy_directory, fixture_repo, repository_root, FixtureRepo};

const SCRATCH_DIRECTORY: &str = "artifacts/tmp/remote-backend";
const DEFAULT_BRANCH: &str = "main";
const PUSHED_BRANCH: &str = "feature/pushed";
const SEED_PATH: &str = "src/app.rs";
const REMOTE_PATH: &str = "src/remote.rs";
const LOCAL_PATH: &str = "src/local.rs";
const UNKNOWN_REMOTE: &str = "no-such-remote";
const SEED_CONTENTS: &str = "fn main() {\n    run();\n}\n";

struct RemoteFixture {
    remote_path: PathBuf,
    producer_path: PathBuf,
    local_path: PathBuf,
}

fn scratch_root(name: &str) -> PathBuf {
    let root = repository_root().join(SCRATCH_DIRECTORY).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("the stale scratch directory is removed");
    }
    fs::create_dir_all(&root).expect("the scratch directory is created");
    root
}

fn run_git_in(working_directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=gitdeck-remote-test",
            "-c",
            "user.email=remote@gitdeck.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .arg("-C")
        .arg(working_directory)
        .args(arguments)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "git {arguments:?} failed in {}: {}",
        working_directory.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn write_file(repository_path: &Path, relative_path: &str, contents: &str) {
    let path = repository_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the tracked file directory is created");
    }
    fs::write(&path, contents)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
}

fn remote_fixture(name: &str) -> RemoteFixture {
    let root = scratch_root(name);
    let remote_path = root.join("remote.git");
    fs::create_dir_all(&remote_path).expect("the bare remote directory is created");
    run_git_in(&remote_path, &["init", "--bare", "--quiet"]);
    let producer_path = root.join("producer");
    fs::create_dir_all(&producer_path).expect("the producer directory is created");
    run_git_in(&producer_path, &["init", "--quiet"]);
    write_file(&producer_path, SEED_PATH, SEED_CONTENTS);
    run_git_in(&producer_path, &["add", "--all"]);
    run_git_in(&producer_path, &["commit", "-q", "-m", "seed the remote"]);
    run_git_in(
        &producer_path,
        &["remote", "add", "origin", path_argument(&remote_path)],
    );
    run_git_in(
        &producer_path,
        &["push", "-q", "-u", "origin", DEFAULT_BRANCH],
    );
    let local_path = root.join("local");
    run_git_in(
        &root,
        &[
            "clone",
            "--quiet",
            path_argument(&remote_path),
            path_argument(&local_path),
        ],
    );
    RemoteFixture {
        remote_path,
        producer_path,
        local_path,
    }
}

fn path_argument(path: &Path) -> &str {
    path.to_str().expect("the scratch path is valid UTF-8")
}

fn commit_in_producer(fixture: &RemoteFixture, subject: &str, contents: &str) -> String {
    write_file(&fixture.producer_path, REMOTE_PATH, contents);
    run_git_in(&fixture.producer_path, &["add", "--all"]);
    run_git_in(&fixture.producer_path, &["commit", "-q", "-m", subject]);
    run_git_in(
        &fixture.producer_path,
        &["push", "-q", "origin", DEFAULT_BRANCH],
    );
    head_commit_of(&fixture.producer_path)
}

fn commit_in_local(repository_path: &Path, subject: &str, contents: &str) {
    write_file(repository_path, LOCAL_PATH, contents);
    run_git_in(repository_path, &["add", "--all"]);
    run_git_in(repository_path, &["commit", "-q", "-m", subject]);
}

fn head_commit_of(repository_path: &Path) -> String {
    run_git_in(repository_path, &["rev-parse", "HEAD"])
        .trim()
        .to_string()
}

fn opened(fixture: &RemoteFixture) -> CliGit {
    CliGit::open(&fixture.local_path).expect("the clone opens as a repository")
}

fn command_failed_message(error: GitError, expected_arguments_prefix: &str) -> String {
    match error {
        GitError::CommandFailed {
            arguments, message, ..
        } => {
            assert!(
                arguments.starts_with(expected_arguments_prefix),
                "the failure names the {expected_arguments_prefix} command, got {arguments:?}"
            );
            assert!(!message.is_empty(), "the failure carries git stderr");
            message
        }
        other => panic!("expected CommandFailed, got {other:?}"),
    }
}

#[test]
fn fetch_updates_behind_count() {
    let fixture = remote_fixture("fetch-updates-behind-count");
    let git = opened(&fixture);
    assert_eq!(
        git.head_info().expect("head info read").behind,
        0,
        "a fresh clone is level with the remote"
    );
    commit_in_producer(
        &fixture,
        "advance the remote",
        "fn main() {\n    advanced();\n}\n",
    );
    git.fetch(None).expect("fetch reaches the default remote");
    let head = git.head_info().expect("head info read");
    assert!(
        head.behind > 0,
        "the fetched commit shows up as behind, head was {head:?}"
    );
    assert_eq!(head.ahead, 0, "the clone has nothing to push");
}

#[test]
fn pull_fast_forward_only_advances_the_branch() {
    let fixture = remote_fixture("pull-fast-forward-only");
    let git = opened(&fixture);
    let remote_head = commit_in_producer(
        &fixture,
        "advance the remote",
        "fn main() {\n    advanced();\n}\n",
    );
    git.pull(PullStrategy::FastForwardOnly)
        .expect("a fast-forward pull succeeds");
    assert_eq!(
        head_commit_of(&fixture.local_path),
        remote_head,
        "the local branch moved to the remote commit"
    );
}

#[test]
fn pull_fast_forward_only_fails_when_diverged() {
    let fixture = remote_fixture("pull-fast-forward-only-diverged");
    let git = opened(&fixture);
    commit_in_producer(&fixture, "remote side", "fn main() {\n    remote();\n}\n");
    commit_in_local(
        &fixture.local_path,
        "local side",
        "fn main() {\n    local();\n}\n",
    );
    let error = git
        .pull(PullStrategy::FastForwardOnly)
        .expect_err("a diverged branch cannot fast-forward");
    let message = command_failed_message(error, "pull ");
    assert!(
        message.contains("fast-forward") || message.contains("diverged"),
        "the failure explains the divergence, message was {message:?}"
    );
}

#[test]
fn pull_merge_creates_a_merge_commit_when_diverged() {
    let fixture = remote_fixture("pull-merge-diverged");
    let git = opened(&fixture);
    commit_in_producer(&fixture, "remote side", "fn main() {\n    remote();\n}\n");
    commit_in_local(
        &fixture.local_path,
        "local side",
        "fn main() {\n    local();\n}\n",
    );
    git.pull(PullStrategy::Merge)
        .expect("a merge pull succeeds on diverged branches");
    let parents = run_git_in(&fixture.local_path, &["log", "-1", "--format=%P"]);
    assert_eq!(
        parents.split_whitespace().count(),
        2,
        "the pull created a merge commit, parents were {parents:?}"
    );
    let count = run_git_in(&fixture.local_path, &["rev-list", "--count", "HEAD"]);
    assert_eq!(count.trim(), "4", "seed plus both sides plus the merge");
}

#[test]
fn push_request_sets_upstream_for_a_new_branch() {
    let fixture = remote_fixture("push-sets-upstream");
    let git = opened(&fixture);
    run_git_in(
        &fixture.local_path,
        &["checkout", "-q", "-b", PUSHED_BRANCH],
    );
    commit_in_local(
        &fixture.local_path,
        "add the pushed branch",
        "fn main() {\n    pushed();\n}\n",
    );
    git.push_request(PushRequest {
        branch: PUSHED_BRANCH.to_string(),
        set_upstream: true,
        force_with_lease: false,
    })
    .expect("pushing a new branch with an upstream succeeds");
    let remote_branches = run_git_in(&fixture.remote_path, &["branch", "--list", PUSHED_BRANCH]);
    assert!(
        remote_branches.contains(PUSHED_BRANCH),
        "the bare remote holds {PUSHED_BRANCH}, it reported {remote_branches:?}"
    );
    assert_eq!(
        git.head_info().expect("head info read").upstream,
        Some(format!("origin/{PUSHED_BRANCH}")),
        "the local branch tracks the pushed remote branch"
    );
}

#[test]
fn push_request_rejects_a_non_fast_forward_without_force() {
    let fixture = remote_fixture("push-rejects-non-fast-forward");
    let git = opened(&fixture);
    commit_in_local(
        &fixture.local_path,
        "local side",
        "fn main() {\n    local();\n}\n",
    );
    commit_in_producer(&fixture, "remote side", "fn main() {\n    remote();\n}\n");
    let error = git
        .push_request(PushRequest {
            branch: DEFAULT_BRANCH.to_string(),
            set_upstream: false,
            force_with_lease: false,
        })
        .expect_err("a diverged branch cannot be pushed without force");
    let message = command_failed_message(error, "push ");
    assert!(
        message.contains("rejected") || message.contains("non-fast-forward"),
        "the failure names the rejection, message was {message:?}"
    );
}

#[test]
fn push_request_force_with_lease_overwrites_the_remote() {
    let fixture = remote_fixture("push-force-with-lease");
    let git = opened(&fixture);
    write_file(
        &fixture.local_path,
        SEED_PATH,
        "fn main() {\n    rewritten();\n}\n",
    );
    run_git_in(
        &fixture.local_path,
        &[
            "commit",
            "-q",
            "-a",
            "--amend",
            "-m",
            "rewrite the seed commit",
        ],
    );
    let rewritten_head = head_commit_of(&fixture.local_path);
    git.push_request(PushRequest {
        branch: DEFAULT_BRANCH.to_string(),
        set_upstream: false,
        force_with_lease: true,
    })
    .expect("force-with-lease replaces the rewritten branch");
    assert_eq!(
        run_git_in(&fixture.remote_path, &["rev-parse", DEFAULT_BRANCH])
            .trim()
            .to_string(),
        rewritten_head,
        "the bare remote points at the rewritten commit"
    );
}

#[test]
fn fetch_without_a_remote_reports_no_remote_to_fetch_from() {
    let root = scratch_root("fetch-without-a-remote");
    let repository_path = root.join("repository");
    copy_directory(&fixture_repo(FixtureRepo::Dirty), &repository_path)
        .expect("the fixture repository is copied");
    let git = CliGit::open(&repository_path).expect("the copy opens as a repository");
    let fetch_error = git
        .fetch(None)
        .expect_err("a repository without remotes cannot fetch");
    assert_eq!(
        fetch_error.to_string(),
        "the repository has no remote to fetch from"
    );
    let pull_error = git
        .pull(PullStrategy::Merge)
        .expect_err("a repository without remotes cannot pull");
    assert_eq!(
        pull_error.to_string(),
        "the repository has no remote to pull from",
        "pull names pulling instead of pushing"
    );
}

#[test]
fn fetch_reports_a_missing_remote_as_a_command_failure() {
    let fixture = remote_fixture("fetch-missing-remote");
    let git = opened(&fixture);
    let error = git
        .fetch(Some(UNKNOWN_REMOTE))
        .expect_err("an unknown remote name cannot be fetched");
    command_failed_message(error, "fetch ");
    let unreachable_path = fixture.local_path.join("no-such-remote.git");
    run_git_in(
        &fixture.local_path,
        &[
            "remote",
            "set-url",
            "origin",
            path_argument(&unreachable_path),
        ],
    );
    let error = git
        .fetch(None)
        .expect_err("an unreachable remote URL cannot be fetched");
    let message = command_failed_message(error, "fetch ");
    assert!(
        message.contains("repository") || message.contains("does not exist"),
        "the failure explains the unreachable remote, message was {message:?}"
    );
}

#[test]
fn fetch_from_a_refused_ssh_remote_reports_a_command_failure() {
    let fixture = remote_fixture("fetch-refused-ssh-remote");
    let git = opened(&fixture);
    run_git_in(
        &fixture.local_path,
        &[
            "config",
            "core.sshCommand",
            "ssh -o BatchMode=yes -o ConnectTimeout=2",
        ],
    );
    run_git_in(
        &fixture.local_path,
        &[
            "remote",
            "set-url",
            "origin",
            "ssh://git@127.0.0.1:1/gitdeck-nonexistent.git",
        ],
    );
    let error = git
        .fetch(None)
        .expect_err("a refused ssh remote cannot be fetched");
    let message = command_failed_message(error, "fetch ");
    assert!(
        message.contains("repository") || message.contains("Permission") || message.contains("ssh"),
        "the failure explains the refused connection, message was {message:?}"
    );
}

#[test]
fn pull_rebase_replays_the_local_commit_on_the_remote() {
    let fixture = remote_fixture("pull-rebase-diverged");
    let git = opened(&fixture);
    let remote_head =
        commit_in_producer(&fixture, "remote side", "fn main() {\n    remote();\n}\n");
    commit_in_local(
        &fixture.local_path,
        "local side",
        "fn main() {\n    local();\n}\n",
    );
    git.pull(PullStrategy::Rebase)
        .expect("a rebase pull succeeds on diverged branches");
    let parents = run_git_in(&fixture.local_path, &["log", "-1", "--format=%P"]);
    assert_eq!(
        parents.split_whitespace().count(),
        1,
        "the rebase produced a linear history, parents were {parents:?}"
    );
    let replayed_base = run_git_in(&fixture.local_path, &["rev-parse", "HEAD^"]);
    assert_eq!(
        replayed_base.trim(),
        remote_head,
        "the local commit now sits on the remote commit"
    );
}
