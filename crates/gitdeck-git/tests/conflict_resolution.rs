use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gitdeck_git::{CliGit, ConflictResolution, GitBackend, GitError};
use gitdeck_testkit::repository_root;

const SCRATCH_DIRECTORY: &str = "artifacts/tmp/conflict-resolution";
const CONFLICT_PATH: &str = "src/conflict.txt";
const BASE_CONTENT: &str = "first line\nshared line\nlast line\n";
const OUR_CONTENT: &str = "first line\nmain line\nlast line\n";
const THEIR_CONTENT: &str = "first line\nfeature line\nlast line\n";
const FEATURE_BRANCH: &str = "feature";
const MAIN_BRANCH: &str = "main";
const BASE_SUBJECT: &str = "base commit";
const MAIN_SUBJECT: &str = "main branch change";
const FEATURE_SUBJECT: &str = "feature branch change";
const CONFLICT_MARKER: &str = "<<<<<<<";
const UNMERGED_STATUS_PREFIX: &str = "UU ";
const LEADING_DASH_PATH: &str = "-src/conflict.txt";

fn run_git(working_directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=gitdeck-conflict-test",
            "-c",
            "user.email=conflict@gitdeck.invalid",
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

fn run_git_expecting_failure(working_directory: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=gitdeck-conflict-test",
            "-c",
            "user.email=conflict@gitdeck.invalid",
            "-c",
            "commit.gpgsign=false",
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
        !output.status.success(),
        "git {arguments:?} unexpectedly succeeded in {}",
        working_directory.display()
    );
}

fn write_file(repository_path: &Path, contents: &str) {
    let path = repository_path.join(CONFLICT_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the parent directory is created");
    }
    fs::write(&path, contents).expect("the conflicted path is written");
}

fn read_file(repository_path: &Path) -> String {
    fs::read_to_string(repository_path.join(CONFLICT_PATH))
        .expect("the conflicted path is readable")
}

fn conflicted_repository(name: &str) -> PathBuf {
    let repository_path = repository_root().join(SCRATCH_DIRECTORY).join(name);
    if repository_path.exists() {
        fs::remove_dir_all(&repository_path).expect("the stale scratch repository is removed");
    }
    fs::create_dir_all(&repository_path).expect("the scratch repository is created");
    run_git(&repository_path, &["init", "-q"]);
    write_file(&repository_path, BASE_CONTENT);
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", BASE_SUBJECT]);
    run_git(&repository_path, &["switch", "-q", "-c", FEATURE_BRANCH]);
    write_file(&repository_path, THEIR_CONTENT);
    run_git(
        &repository_path,
        &["commit", "-q", "-a", "-m", FEATURE_SUBJECT],
    );
    run_git(&repository_path, &["switch", "-q", MAIN_BRANCH]);
    write_file(&repository_path, OUR_CONTENT);
    run_git(
        &repository_path,
        &["commit", "-q", "-a", "-m", MAIN_SUBJECT],
    );
    run_git_expecting_failure(&repository_path, &["merge", "--no-edit", FEATURE_BRANCH]);
    assert!(
        read_file(&repository_path).contains(CONFLICT_MARKER),
        "the merge leaves conflict markers in the work tree"
    );
    repository_path
}

fn status_porcelain(repository_path: &Path) -> String {
    run_git(repository_path, &["status", "--porcelain"])
}

fn open(repository_path: &Path) -> CliGit {
    CliGit::open(repository_path).expect("the conflicted repository opens")
}

#[test]
fn resolve_conflict_with_ours_keeps_our_content() {
    let repository_path = conflicted_repository("ours");
    let git = open(&repository_path);
    git.resolve_conflict(CONFLICT_PATH, ConflictResolution::Ours)
        .expect("accepting our side succeeds");
    assert_eq!(read_file(&repository_path), OUR_CONTENT);
    let status = status_porcelain(&repository_path);
    assert!(
        !status.contains(UNMERGED_STATUS_PREFIX),
        "the path stays conflicted: {status}"
    );
    assert!(
        !status.contains(CONFLICT_PATH),
        "our side matches the head commit, so the path leaves the change list: {status}"
    );
}

#[test]
fn resolve_conflict_with_theirs_keeps_their_content() {
    let repository_path = conflicted_repository("theirs");
    let git = open(&repository_path);
    git.resolve_conflict(CONFLICT_PATH, ConflictResolution::Theirs)
        .expect("accepting their side succeeds");
    assert_eq!(read_file(&repository_path), THEIR_CONTENT);
    let status = status_porcelain(&repository_path);
    assert!(
        !status.contains(UNMERGED_STATUS_PREFIX),
        "the path stays conflicted: {status}"
    );
    assert!(
        status.contains("M  "),
        "the accepted side is staged in the index: {status}"
    );
}

#[test]
fn mark_resolved_stages_the_conflicted_file_without_touching_content() {
    let repository_path = conflicted_repository("mark-resolved");
    let git = open(&repository_path);
    let conflicted_content = read_file(&repository_path);
    git.resolve_conflict(CONFLICT_PATH, ConflictResolution::MarkResolved)
        .expect("marking the path resolved succeeds");
    assert_eq!(read_file(&repository_path), conflicted_content);
    assert!(read_file(&repository_path).contains(CONFLICT_MARKER));
    let status = status_porcelain(&repository_path);
    assert!(
        !status.contains(UNMERGED_STATUS_PREFIX),
        "the path stays conflicted: {status}"
    );
    assert!(
        status.contains("M  "),
        "the resolved path is staged in the index: {status}"
    );
}

#[test]
fn resolve_conflict_refuses_a_clean_path() {
    let repository_path = conflicted_repository("clean-path");
    write_file(&repository_path, CONFLICT_PATH);
    run_git(&repository_path, &["add", "--", CONFLICT_PATH]);
    let clean_content = read_file(&repository_path);
    let git = open(&repository_path);
    let result = git.resolve_conflict(CONFLICT_PATH, ConflictResolution::Ours);
    match result {
        Err(GitError::NotConflicted { path }) => assert_eq!(path, CONFLICT_PATH),
        other => panic!("a clean path must be refused, got {other:?}"),
    }
    assert_eq!(read_file(&repository_path), clean_content);
    assert!(
        !status_porcelain(&repository_path).contains(UNMERGED_STATUS_PREFIX),
        "the refused call writes nothing"
    );
}

#[test]
fn resolve_conflict_refuses_a_path_with_a_leading_dash() {
    let repository_path = conflicted_repository("leading-dash");
    let git = open(&repository_path);
    let result = git.resolve_conflict(LEADING_DASH_PATH, ConflictResolution::Ours);
    match result {
        Err(GitError::UnsafePathspec { path }) => assert_eq!(path, LEADING_DASH_PATH),
        other => panic!("a leading dash path must be refused, got {other:?}"),
    }
    assert!(
        status_porcelain(&repository_path).contains(UNMERGED_STATUS_PREFIX),
        "the refused call leaves the real conflict untouched"
    );
}
