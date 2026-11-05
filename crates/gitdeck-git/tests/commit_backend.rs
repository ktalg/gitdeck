use std::path::{Path, PathBuf};

use gitdeck_core::{LogFilter, LogQuery};
use gitdeck_git::{CliGit, CommitOutcome, CommitRequest, GitBackend, GitError};
use gitdeck_testkit::{
    copy_directory, fixture_git_command, fixture_repo, scratch_directory, FixtureRepo,
};

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/commit-backend";

fn dirty_fixture() -> PathBuf {
    fixture_repo(FixtureRepo::Dirty)
}

fn scratch_dirty_repository(name: &str) -> PathBuf {
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    copy_directory(&dirty_fixture(), &target).expect("the fixture repository is copied");
    target
}

fn run_git(repository_path: &Path, arguments: &[&str]) -> String {
    let output = fixture_git_command(repository_path)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn commit_request(message: &str, paths: &[&str]) -> CommitRequest {
    CommitRequest {
        message: message.to_string(),
        amend: false,
        sign_off: false,
        paths: paths.iter().map(|path| path.to_string()).collect(),
        hunks: Vec::new(),
    }
}

fn changed_paths(git: &CliGit) -> Vec<String> {
    git.changes()
        .expect("changes read")
        .into_iter()
        .map(|change| change.path)
        .collect()
}

#[test]
fn commit_rejects_an_empty_message_without_touching_the_repository() {
    let repository_path = scratch_dirty_repository("empty-message");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let head_before = run_git(&repository_path, &["rev-parse", "HEAD"]);
    let error = git
        .commit(commit_request("   ", &["notes.txt"]))
        .expect_err("an empty message is rejected");
    assert!(matches!(error, GitError::EmptyCommitMessage), "{error:?}");
    assert_eq!(
        run_git(&repository_path, &["rev-parse", "HEAD"]),
        head_before
    );
    assert!(changed_paths(&git).contains(&String::from("notes.txt")));
}

#[test]
fn commit_rejects_an_empty_path_list_without_touching_the_repository() {
    let repository_path = scratch_dirty_repository("empty-paths");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let head_before = run_git(&repository_path, &["rev-parse", "HEAD"]);
    let error = git
        .commit(commit_request("a message", &[]))
        .expect_err("an empty path list is rejected");
    assert!(matches!(error, GitError::NothingToCommit), "{error:?}");
    assert_eq!(
        run_git(&repository_path, &["rev-parse", "HEAD"]),
        head_before
    );
}

#[test]
fn commit_takes_only_the_requested_paths_and_reports_the_new_head() {
    let repository_path = scratch_dirty_repository("selected-paths");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let outcome = git
        .commit(commit_request("commit notes only", &["notes.txt"]))
        .expect("the commit succeeds");
    assert_eq!(
        outcome.commit_id,
        run_git(&repository_path, &["rev-parse", "HEAD"]).trim()
    );
    assert_eq!(
        run_git(&repository_path, &["log", "-1", "--format=%s"]).trim(),
        "commit notes only"
    );
    let mut committed_paths: Vec<String> = run_git(
        &repository_path,
        &["show", "--name-only", "--format=", "HEAD"],
    )
    .lines()
    .map(str::to_string)
    .collect();
    committed_paths.sort();
    assert_eq!(
        committed_paths,
        vec![String::from("notes.txt")],
        "the commit takes the requested paths and leaves the other staged paths behind"
    );
    let mut remaining = changed_paths(&git);
    remaining.sort();
    assert_eq!(
        remaining,
        vec![
            String::from("docs/readme.md"),
            String::from("src/app.rs"),
            String::from("src/module.rs"),
        ],
        "the committed path leaves the change list and the pre-staged paths stay"
    );
}

#[test]
fn commit_appends_the_sign_off_trailer() {
    let repository_path = scratch_dirty_repository("sign-off");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let request = CommitRequest {
        sign_off: true,
        ..commit_request("signed commit", &["notes.txt"])
    };
    git.commit(request).expect("the signed commit succeeds");
    let message = run_git(&repository_path, &["log", "-1", "--format=%B"]);
    assert!(
        message.contains("Signed-off-by: gitdeck-fixture <fixture@gitdeck.invalid>"),
        "the trailer is missing: {message:?}"
    );
}

#[test]
fn amend_rewrites_the_head_commit_without_adding_one() {
    let repository_path = scratch_dirty_repository("amend");
    let git = CliGit::open(&repository_path).expect("repository opens");
    git.commit(commit_request("first message", &["notes.txt"]))
        .expect("the first commit succeeds");
    let commit_count_before = run_git(&repository_path, &["rev-list", "--count", "HEAD"]);
    let request = CommitRequest {
        amend: true,
        ..commit_request("amended message", &["src/app.rs"])
    };
    let outcome = git.commit(request).expect("the amended commit succeeds");
    assert_eq!(
        run_git(&repository_path, &["rev-list", "--count", "HEAD"]),
        commit_count_before
    );
    assert_eq!(
        run_git(&repository_path, &["log", "-1", "--format=%s"]).trim(),
        "amended message"
    );
    assert_eq!(
        outcome.commit_id,
        run_git(&repository_path, &["rev-parse", "HEAD"]).trim()
    );
}

#[test]
fn amend_without_paths_rewrites_the_head_message_only() {
    let repository_path = scratch_dirty_repository("amend-message-only");
    let git = CliGit::open(&repository_path).expect("repository opens");
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", "clean work tree"]);
    let commit_count_before = run_git(&repository_path, &["rev-list", "--count", "HEAD"]);
    let tree_before = run_git(&repository_path, &["rev-parse", "HEAD^{tree}"]);
    let request = CommitRequest {
        amend: true,
        ..commit_request("reworded head", &[])
    };
    let outcome = git
        .commit(request)
        .expect("the message-only amend succeeds");
    assert_eq!(
        run_git(&repository_path, &["rev-list", "--count", "HEAD"]),
        commit_count_before,
        "the amend keeps the commit count"
    );
    assert_eq!(
        run_git(&repository_path, &["log", "-1", "--format=%s"]).trim(),
        "reworded head"
    );
    assert_eq!(
        run_git(&repository_path, &["rev-parse", "HEAD^{tree}"]),
        tree_before,
        "the amend touches the message only"
    );
    assert_eq!(
        outcome.commit_id,
        run_git(&repository_path, &["rev-parse", "HEAD"]).trim()
    );
    assert!(
        changed_paths(&git).is_empty(),
        "the clean work tree stays clean"
    );
}

#[test]
fn last_commit_message_returns_the_full_head_message() {
    let git = CliGit::open(dirty_fixture()).expect("repository opens");
    assert_eq!(
        git.last_commit_message().expect("message read"),
        "call run twice"
    );
}

#[test]
fn push_without_a_remote_reports_no_remote() {
    let repository_path = scratch_dirty_repository("push-without-remote");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let error = git.push("main").expect_err("the fixture has no remote");
    assert!(matches!(error, GitError::NoRemote), "{error:?}");
    let commits = git
        .log(
            LogQuery {
                max_commits: 1,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("log read");
    assert_eq!(commits[0].subject, "call run twice");
}

#[test]
fn commit_outcome_compares_by_commit_id() {
    let outcome = CommitOutcome {
        commit_id: String::from("abc"),
    };
    assert_eq!(
        outcome,
        CommitOutcome {
            commit_id: String::from("abc")
        }
    );
}
