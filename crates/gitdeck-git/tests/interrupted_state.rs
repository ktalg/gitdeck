use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_core::RepoOperation;
use gitdeck_git::{CliGit, GitBackend, GitError};
use gitdeck_testkit::fixture_git_command;
use gitdeck_testkit::repository_root;

const SCRATCH_DIRECTORY: &str = "artifacts/tmp/interrupted-state";
const MAIN_BRANCH: &str = "main";
const FEATURE_BRANCH: &str = "feature";
const SHARED_PATH: &str = "src/shared.txt";
const BASE_CONTENT: &str = "first line\nshared line\nlast line\n";
const OUR_CONTENT: &str = "first line\nmain line\nlast line\n";
const THEIR_CONTENT: &str = "first line\nfeature line\nlast line\n";
const RESOLVED_CONTENT: &str = "first line\nresolved line\nlast line\n";
const CLEAN_PATH: &str = "src/clean.txt";
const CLEAN_BASE_CONTENT: &str = "clean base\n";
const CLEAN_THEIR_CONTENT: &str = "clean feature\n";
const LOCAL_EDIT_CONTENT: &str = "my own edit\n";

fn run_git(working_directory: &Path, arguments: &[&str]) -> String {
    let output = fixture_git_command(working_directory)
        .args(["-c", "init.defaultBranch=main"])
        .args(arguments)
        .env("GIT_EDITOR", "true")
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

fn run_git_expecting_failure(working_directory: &Path, arguments: &[&str]) {
    let output = fixture_git_command(working_directory)
        .args(arguments)
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        !output.status.success(),
        "git {arguments:?} was expected to stop in {}",
        working_directory.display()
    );
}

fn write_file(repository_path: &Path, relative_path: &str, contents: &str) {
    let path = repository_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the parent directory is created");
    }
    fs::write(&path, contents).expect("the scratch file is written");
}

fn scratch_repository(name: &str) -> PathBuf {
    let repository_path = repository_root().join(SCRATCH_DIRECTORY).join(name);
    if repository_path.exists() {
        fs::remove_dir_all(&repository_path).expect("the stale scratch repository is removed");
    }
    fs::create_dir_all(&repository_path).expect("the scratch repository is created");
    run_git(&repository_path, &["init", "-q", "-b", MAIN_BRANCH]);
    run_git(
        &repository_path,
        &["config", "user.name", "gitdeck-interrupted-test"],
    );
    run_git(
        &repository_path,
        &["config", "user.email", "interrupted@gitdeck.invalid"],
    );
    write_file(&repository_path, SHARED_PATH, BASE_CONTENT);
    write_file(&repository_path, CLEAN_PATH, CLEAN_BASE_CONTENT);
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", "base commit"]);
    repository_path
}

fn diverged_repository(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    run_git(&repository_path, &["switch", "-q", "-c", FEATURE_BRANCH]);
    write_file(&repository_path, SHARED_PATH, THEIR_CONTENT);
    write_file(&repository_path, CLEAN_PATH, CLEAN_THEIR_CONTENT);
    run_git(
        &repository_path,
        &["commit", "-q", "-a", "-m", "feature change"],
    );
    run_git(&repository_path, &["switch", "-q", MAIN_BRANCH]);
    write_file(&repository_path, SHARED_PATH, OUR_CONTENT);
    run_git(
        &repository_path,
        &["commit", "-q", "-a", "-m", "main change"],
    );
    repository_path
}

fn repository_in_a_merge_conflict(name: &str) -> PathBuf {
    let repository_path = diverged_repository(name);
    run_git_expecting_failure(&repository_path, &["merge", FEATURE_BRANCH]);
    repository_path
}

fn repository_in_a_rebase_conflict(name: &str) -> PathBuf {
    let repository_path = diverged_repository(name);
    run_git(&repository_path, &["switch", "-q", FEATURE_BRANCH]);
    run_git_expecting_failure(&repository_path, &["rebase", MAIN_BRANCH]);
    repository_path
}

fn repository_in_a_cherry_pick_conflict(name: &str) -> PathBuf {
    let repository_path = diverged_repository(name);
    let feature_commit = run_git(&repository_path, &["rev-parse", FEATURE_BRANCH]);
    run_git_expecting_failure(&repository_path, &["cherry-pick", feature_commit.trim()]);
    repository_path
}

fn repository_in_a_revert_conflict(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    write_file(&repository_path, SHARED_PATH, OUR_CONTENT);
    run_git(
        &repository_path,
        &["commit", "-q", "-a", "-m", "main change"],
    );
    write_file(&repository_path, SHARED_PATH, THEIR_CONTENT);
    run_git(
        &repository_path,
        &["commit", "-q", "-a", "-m", "later change"],
    );
    run_git_expecting_failure(&repository_path, &["revert", "--no-edit", "HEAD~1"]);
    repository_path
}

fn head_commit_id(repository_path: &Path) -> String {
    run_git(repository_path, &["rev-parse", "HEAD"])
        .trim()
        .to_string()
}

fn status_porcelain(repository_path: &Path) -> String {
    run_git(repository_path, &["status", "--porcelain"])
}

#[test]
fn head_info_reports_each_operation() {
    for (name, build, operation) in [
        (
            "operation-merge",
            repository_in_a_merge_conflict as fn(&str) -> PathBuf,
            RepoOperation::Merging,
        ),
        (
            "operation-rebase",
            repository_in_a_rebase_conflict as fn(&str) -> PathBuf,
            RepoOperation::Rebasing,
        ),
        (
            "operation-cherry-pick",
            repository_in_a_cherry_pick_conflict as fn(&str) -> PathBuf,
            RepoOperation::CherryPicking,
        ),
        (
            "operation-revert",
            repository_in_a_revert_conflict as fn(&str) -> PathBuf,
            RepoOperation::Reverting,
        ),
    ] {
        let repository_path = build(name);
        let git = CliGit::open(&repository_path).expect("the scratch repository opens");
        let head = git.head_info().expect("the head reads");
        assert_eq!(head.operation, Some(operation), "{name} operation");
        assert_eq!(
            head.merging,
            operation == RepoOperation::Merging,
            "{name} merging flag"
        );
    }
}

#[test]
fn abort_merge_restores_the_previous_head() {
    let repository_path = diverged_repository("abort-merge");
    let head_before = head_commit_id(&repository_path);
    run_git_expecting_failure(&repository_path, &["merge", FEATURE_BRANCH]);
    let git = CliGit::open(&repository_path).expect("the scratch repository opens");
    git.abort_operation().expect("the merge aborts");
    assert_eq!(head_commit_id(&repository_path), head_before);
    assert!(status_porcelain(&repository_path).trim().is_empty());
    assert_eq!(git.head_info().expect("the head reads").operation, None);
}

#[test]
fn abort_rebase_restores_the_previous_head() {
    let repository_path = diverged_repository("abort-rebase");
    let head_before = head_commit_id(&repository_path);
    run_git_expecting_failure(&repository_path, &["rebase", FEATURE_BRANCH]);
    let git = CliGit::open(&repository_path).expect("the scratch repository opens");
    git.abort_operation().expect("the rebase aborts");
    assert_eq!(head_commit_id(&repository_path), head_before);
    assert!(status_porcelain(&repository_path).trim().is_empty());
    assert_eq!(git.head_info().expect("the head reads").operation, None);
}

#[test]
fn continue_rebase_after_resolving() {
    let repository_path = repository_in_a_rebase_conflict("continue-rebase");
    let git = CliGit::open(&repository_path).expect("the scratch repository opens");
    write_file(&repository_path, SHARED_PATH, RESOLVED_CONTENT);
    run_git(&repository_path, &["add", "--all"]);
    git.continue_operation().expect("the rebase continues");
    assert_eq!(git.head_info().expect("the head reads").operation, None);
    assert_eq!(status_porcelain(&repository_path).trim(), "");
    let feature_commit = run_git(&repository_path, &["log", "-1", "--format=%s"]);
    assert_eq!(feature_commit.trim(), "feature change");
    let picked = run_git(&repository_path, &["show", "HEAD:src/shared.txt"]);
    assert_eq!(picked, RESOLVED_CONTENT);
}

#[test]
fn abort_without_an_operation_reports_an_error() {
    let repository_path = scratch_repository("abort-without-operation");
    let git = CliGit::open(&repository_path).expect("the scratch repository opens");
    assert!(matches!(
        git.abort_operation(),
        Err(GitError::NoOperationInProgress)
    ));
    assert!(matches!(
        git.continue_operation(),
        Err(GitError::NoOperationInProgress)
    ));
}

#[test]
fn abort_with_a_dirty_worktree_reports_the_git_error() {
    let repository_path = repository_in_a_merge_conflict("abort-dirty-worktree");
    write_file(&repository_path, CLEAN_PATH, LOCAL_EDIT_CONTENT);
    let git = CliGit::open(&repository_path).expect("the scratch repository opens");
    let error = git.abort_operation().expect_err("the abort fails");
    let message = error.to_string();
    assert!(
        message.contains("git merge --abort failed"),
        "the error names the failed command: {message}"
    );
    assert!(
        message.contains("not uptodate"),
        "the error keeps the git reason: {message}"
    );
    assert_eq!(
        git.head_info().expect("the head reads").operation,
        Some(RepoOperation::Merging),
        "the merge stays in progress after a failed abort"
    );
}
