use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gitdeck_core::{CommitSummary, LogFilter, LogQuery};
use gitdeck_git::{CliGit, GitBackend, GitError};
use gitdeck_testkit::scratch_directory;

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/commit-meta";
const CURRENT_BRANCH: &str = "main";
const AUTHOR_NAME: &str = "Alice Author";
const AUTHOR_EMAIL: &str = "alice@gitdeck.invalid";
const COMMITTER_NAME: &str = "Someone Else";
const COMMITTER_EMAIL: &str = "else@gitdeck.invalid";
const UNKNOWN_COMMIT_ID: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
const UNBOUNDED_COMMITS: usize = 1_000;

fn scratch_repository(name: &str) -> PathBuf {
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    fs::create_dir_all(&target).expect("the scratch repository directory is created");
    let target = target.canonicalize().expect("the scratch path resolves");
    run_git(&target, &["init", "-q", "-b", CURRENT_BRANCH]);
    target
}

fn run_git(repository_path: &Path, arguments: &[&str]) -> String {
    run_git_with_environment(repository_path, arguments, &[])
}

fn run_git_with_environment(
    repository_path: &Path,
    arguments: &[&str],
    environment: &[(&str, &str)],
) -> String {
    let name_argument = format!("user.name={AUTHOR_NAME}");
    let email_argument = format!("user.email={AUTHOR_EMAIL}");
    let mut command = Command::new("git");
    command
        .args([
            "-c",
            name_argument.as_str(),
            "-c",
            email_argument.as_str(),
            "-c",
            "commit.gpgsign=false",
        ])
        .arg("-C")
        .arg(repository_path)
        .args(arguments)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C");
    for (variable, value) in environment {
        command.env(variable, value);
    }
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "git {arguments:?} failed in {}: {}",
        repository_path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn commit_subject(repository_path: &Path, subject: &str) -> String {
    run_git(
        repository_path,
        &["commit", "-q", "--allow-empty", "-m", subject],
    );
    head_commit_id(repository_path)
}

fn head_commit_id(repository_path: &Path) -> String {
    run_git(repository_path, &["rev-parse", "HEAD"])
        .trim()
        .to_string()
}

fn logged_commits(repository_path: &Path) -> Vec<CommitSummary> {
    CliGit::open(repository_path)
        .expect("the scratch repository opens")
        .log(
            LogQuery {
                max_commits: UNBOUNDED_COMMITS,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("log read")
}

#[test]
fn committer_differs_from_author_after_a_rebase() {
    let repository_path = scratch_repository("rebase-identity");
    commit_subject(&repository_path, "base commit");
    run_git(&repository_path, &["checkout", "-q", "-b", "topic"]);
    commit_subject(&repository_path, "topic commit");
    run_git(&repository_path, &["checkout", "-q", CURRENT_BRANCH]);
    commit_subject(&repository_path, "main side commit");
    run_git(&repository_path, &["checkout", "-q", "topic"]);
    run_git_with_environment(
        &repository_path,
        &["rebase", "-q", CURRENT_BRANCH],
        &[
            ("GIT_COMMITTER_NAME", COMMITTER_NAME),
            ("GIT_COMMITTER_EMAIL", COMMITTER_EMAIL),
        ],
    );

    let commits = logged_commits(&repository_path);
    let replayed_commit = commits
        .iter()
        .find(|commit| commit.subject == "topic commit")
        .expect("the replayed commit is in the log");
    assert_eq!(replayed_commit.author_name, AUTHOR_NAME);
    assert_eq!(replayed_commit.author_email, AUTHOR_EMAIL);
    assert_eq!(replayed_commit.committer_name, COMMITTER_NAME);
    assert_eq!(replayed_commit.committer_email, COMMITTER_EMAIL);
    assert_ne!(replayed_commit.author_name, replayed_commit.committer_name);
    assert!(replayed_commit.author_timestamp > 0);
    assert!(replayed_commit.committer_timestamp > 0);
}

#[test]
fn branches_containing_lists_head_first_then_locals_then_remotes() {
    let repository_path = scratch_repository("containing-order");
    let commit_id = commit_subject(&repository_path, "shared commit");
    run_git(&repository_path, &["branch", "feature"]);
    run_git(&repository_path, &["branch", "alpha"]);
    run_git(
        &repository_path,
        &["update-ref", "refs/remotes/origin/main", commit_id.as_str()],
    );

    let branches = CliGit::open(&repository_path)
        .expect("the scratch repository opens")
        .branches_containing(&commit_id)
        .expect("containing branches read");
    let names: Vec<&str> = branches.iter().map(|branch| branch.name.as_str()).collect();
    assert_eq!(names, vec!["main", "alpha", "feature", "origin/main"]);
    assert!(branches[0].is_head);
    assert_eq!(branches[0].kind, gitdeck_core::RefKind::LocalBranch);
    assert!(branches[1..3].iter().all(|branch| !branch.is_head));
    assert_eq!(branches[3].kind, gitdeck_core::RefKind::RemoteBranch);
    assert!(!branches[3].is_head);
}

#[test]
fn branches_containing_returns_empty_for_an_orphan_commit() {
    let repository_path = scratch_repository("containing-orphan");
    let base_commit_id = commit_subject(&repository_path, "base commit");
    run_git(&repository_path, &["checkout", "-q", "-b", "discarded"]);
    let orphan_commit_id = commit_subject(&repository_path, "orphan commit");
    run_git(&repository_path, &["checkout", "-q", CURRENT_BRANCH]);
    run_git(&repository_path, &["branch", "-D", "discarded"]);
    assert_eq!(head_commit_id(&repository_path), base_commit_id);

    let branches = CliGit::open(&repository_path)
        .expect("the scratch repository opens")
        .branches_containing(&orphan_commit_id)
        .expect("containing branches read");
    assert!(branches.is_empty(), "{branches:?}");

    let containing_the_base = CliGit::open(&repository_path)
        .expect("the scratch repository opens")
        .branches_containing(&base_commit_id)
        .expect("containing branches read");
    let names: Vec<&str> = containing_the_base
        .iter()
        .map(|branch| branch.name.as_str())
        .collect();
    assert_eq!(names, vec![CURRENT_BRANCH]);
}

#[test]
fn branches_containing_reports_an_error_for_an_unknown_commit() {
    let repository_path = scratch_repository("containing-unknown");
    commit_subject(&repository_path, "base commit");

    let error = CliGit::open(&repository_path)
        .expect("the scratch repository opens")
        .branches_containing(UNKNOWN_COMMIT_ID)
        .expect_err("an unknown commit is rejected");
    match error {
        GitError::CommandFailed { message, .. } => {
            assert!(message.contains("no such commit"), "{message}");
        }
        other => panic!("expected a command failure, got {other:?}"),
    }
}
