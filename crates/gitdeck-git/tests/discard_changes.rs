use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gitdeck_git::{CliGit, GitBackend, GitError};
use gitdeck_testkit::repository_root;

const SCRATCH_DIRECTORY: &str = "artifacts/tmp/discard-changes";
const TRACKED_PATH: &str = "src/tracked.txt";
const UNTRACKED_PATH: &str = "notes.txt";
const DIRECTORY_PATH: &str = "src";
const LEADING_DASH_PATH: &str = "-notes.txt";
const HEAD_CONTENT: &str = "first line\nsecond line\n";
const EDITED_CONTENT: &str = "first line\nedited line\n";
const STAGED_CONTENT: &str = "first line\nstaged line\n";

fn run_git(working_directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=gitdeck-discard-test",
            "-c",
            "user.email=discard@gitdeck.invalid",
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
        fs::create_dir_all(parent).expect("the parent directory is created");
    }
    fs::write(&path, contents).expect("the repository file is written");
}

fn read_file(repository_path: &Path) -> String {
    fs::read_to_string(repository_path.join(TRACKED_PATH)).expect("the tracked path is readable")
}

fn scratch_repository(name: &str) -> PathBuf {
    let repository_path = repository_root().join(SCRATCH_DIRECTORY).join(name);
    if repository_path.exists() {
        fs::remove_dir_all(&repository_path).expect("the stale scratch repository is removed");
    }
    fs::create_dir_all(&repository_path).expect("the scratch repository is created");
    run_git(&repository_path, &["init", "-q"]);
    write_file(&repository_path, TRACKED_PATH, HEAD_CONTENT);
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", "base commit"]);
    repository_path
}

fn status_porcelain(repository_path: &Path) -> String {
    run_git(repository_path, &["status", "--porcelain"])
}

fn open(repository_path: &Path) -> CliGit {
    CliGit::open(repository_path).expect("the scratch repository opens")
}

#[test]
fn discard_changes_restores_a_modified_file() {
    let repository_path = scratch_repository("modified");
    write_file(&repository_path, TRACKED_PATH, EDITED_CONTENT);
    assert!(status_porcelain(&repository_path).contains(TRACKED_PATH));
    let git = open(&repository_path);
    git.discard_changes(TRACKED_PATH)
        .expect("discarding the modification succeeds");
    assert_eq!(read_file(&repository_path), HEAD_CONTENT);
    assert_eq!(
        status_porcelain(&repository_path).trim(),
        "",
        "the work tree is clean again"
    );
}

#[test]
fn discard_changes_removes_an_untracked_file() {
    let repository_path = scratch_repository("untracked");
    write_file(&repository_path, UNTRACKED_PATH, "untracked notes\n");
    assert!(status_porcelain(&repository_path).contains(UNTRACKED_PATH));
    let git = open(&repository_path);
    git.discard_changes(UNTRACKED_PATH)
        .expect("discarding the untracked file succeeds");
    assert!(
        !repository_path.join(UNTRACKED_PATH).exists(),
        "the untracked file is deleted"
    );
    assert_eq!(
        status_porcelain(&repository_path).trim(),
        "",
        "the work tree is clean again"
    );
}

#[test]
fn discard_changes_restores_both_the_index_and_the_worktree() {
    let repository_path = scratch_repository("staged");
    write_file(&repository_path, TRACKED_PATH, STAGED_CONTENT);
    run_git(&repository_path, &["add", "--", TRACKED_PATH]);
    write_file(&repository_path, TRACKED_PATH, EDITED_CONTENT);
    let staged_status = status_porcelain(&repository_path);
    assert!(
        staged_status.contains("MM "),
        "the path is staged and modified before the discard: {staged_status}"
    );
    let git = open(&repository_path);
    git.discard_changes(TRACKED_PATH)
        .expect("discarding the staged and modified file succeeds");
    assert_eq!(read_file(&repository_path), HEAD_CONTENT);
    assert_eq!(
        status_porcelain(&repository_path).trim(),
        "",
        "the index and the work tree are both restored"
    );
}

#[test]
fn discard_changes_refuses_a_clean_path() {
    let repository_path = scratch_repository("clean");
    let git = open(&repository_path);
    let result = git.discard_changes(TRACKED_PATH);
    match result {
        Err(GitError::NothingToDiscard { path }) => assert_eq!(path, TRACKED_PATH),
        other => panic!("a clean path must be refused, got {other:?}"),
    }
    assert_eq!(read_file(&repository_path), HEAD_CONTENT);
    assert_eq!(
        status_porcelain(&repository_path).trim(),
        "",
        "the refused call writes nothing"
    );
}

#[test]
fn discard_changes_refuses_a_directory() {
    let repository_path = scratch_repository("directory");
    let git = open(&repository_path);
    let result = git.discard_changes(DIRECTORY_PATH);
    match result {
        Err(GitError::DiscardPathIsDirectory { path }) => assert_eq!(path, DIRECTORY_PATH),
        other => panic!("a directory must be refused, got {other:?}"),
    }
    assert!(
        repository_path.join(DIRECTORY_PATH).is_dir(),
        "the refused call keeps the directory"
    );
}

#[test]
fn discard_changes_refuses_an_unsafe_pathspec() {
    let repository_path = scratch_repository("unsafe");
    write_file(&repository_path, UNTRACKED_PATH, "untracked notes\n");
    let git = open(&repository_path);
    let result = git.discard_changes(LEADING_DASH_PATH);
    match result {
        Err(GitError::UnsafePathspec { path }) => assert_eq!(path, LEADING_DASH_PATH),
        other => panic!("a leading dash path must be refused, got {other:?}"),
    }
    assert!(
        repository_path.join(UNTRACKED_PATH).exists(),
        "the refused call keeps the untracked file"
    );
}
