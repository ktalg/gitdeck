use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gitdeck_git::{CliGit, GitBackend, GitError};
use gitdeck_testkit::repository_root;

const SCRATCH_DIRECTORY: &str = "artifacts/tmp/delete-branch";
const CURRENT_BRANCH: &str = "main";
const MERGED_BRANCH: &str = "feature/merged";
const UNMERGED_BRANCH: &str = "feature/unmerged";
const REMOTE_BRANCH: &str = "origin/feature/remote";
const BASE_PATH: &str = "src/base.txt";
const BRANCH_PATH: &str = "src/branch.txt";
const UNMERGED_PATH: &str = "src/unmerged.txt";

fn run_git(working_directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=gitdeck-delete-branch-test",
            "-c",
            "user.email=delete-branch@gitdeck.invalid",
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

fn commit_file(repository_path: &Path, relative_path: &str, contents: &str, message: &str) {
    write_file(repository_path, relative_path, contents);
    run_git(repository_path, &["add", "--all"]);
    run_git(repository_path, &["commit", "-q", "-m", message]);
}

fn scratch_repository(name: &str) -> PathBuf {
    let repository_path = repository_root().join(SCRATCH_DIRECTORY).join(name);
    if repository_path.exists() {
        fs::remove_dir_all(&repository_path).expect("the stale scratch repository is removed");
    }
    fs::create_dir_all(&repository_path).expect("the scratch repository is created");
    run_git(&repository_path, &["init", "-q"]);
    commit_file(&repository_path, BASE_PATH, "base\n", "base commit");
    repository_path
}

fn repository_with_a_merged_branch(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    run_git(&repository_path, &["switch", "-q", "-c", MERGED_BRANCH]);
    commit_file(&repository_path, BRANCH_PATH, "branch\n", "branch commit");
    run_git(&repository_path, &["switch", "-q", CURRENT_BRANCH]);
    run_git(&repository_path, &["merge", "-q", "--no-ff", MERGED_BRANCH]);
    repository_path
}

fn repository_with_an_unmerged_branch(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    run_git(&repository_path, &["switch", "-q", "-c", UNMERGED_BRANCH]);
    commit_file(
        &repository_path,
        UNMERGED_PATH,
        "unmerged\n",
        "unmerged commit",
    );
    run_git(&repository_path, &["switch", "-q", CURRENT_BRANCH]);
    repository_path
}

fn branch_names(repository_path: &Path) -> Vec<String> {
    run_git(repository_path, &["branch", "--format=%(refname:short)"])
        .lines()
        .map(str::to_string)
        .collect()
}

fn open(repository_path: &Path) -> CliGit {
    CliGit::open(repository_path).expect("the scratch repository opens")
}

#[test]
fn delete_branch_removes_a_merged_branch() {
    let repository_path = repository_with_a_merged_branch("merged");
    open(&repository_path)
        .delete_branch(MERGED_BRANCH)
        .expect("a merged branch is deleted");
    let names = branch_names(&repository_path);
    assert!(
        !names.iter().any(|name| name == MERGED_BRANCH),
        "the merged branch is gone from {names:?}"
    );
    assert!(names.iter().any(|name| name == CURRENT_BRANCH));
}

#[test]
fn delete_branch_refuses_an_unmerged_branch() {
    let repository_path = repository_with_an_unmerged_branch("unmerged");
    let error = open(&repository_path)
        .delete_branch(UNMERGED_BRANCH)
        .expect_err("git refuses to delete an unmerged branch");
    let message = error.to_string();
    assert!(
        message.contains("not fully merged"),
        "the git refusal reaches the caller verbatim, got {message}"
    );
    assert!(
        branch_names(&repository_path)
            .iter()
            .any(|name| name == UNMERGED_BRANCH),
        "the unmerged branch survives the refusal"
    );
}

#[test]
fn delete_branch_refuses_a_remote_branch() {
    let repository_path = scratch_repository("remote");
    run_git(
        &repository_path,
        &[
            "update-ref",
            &format!("refs/remotes/{REMOTE_BRANCH}"),
            "HEAD",
        ],
    );
    let error = open(&repository_path)
        .delete_branch(REMOTE_BRANCH)
        .expect_err("a remote branch cannot be deleted locally");
    assert!(
        matches!(error, GitError::RemoteBranchNotDeletable { .. }),
        "the remote branch name reports the dedicated error, got {error:?}"
    );
    assert!(error
        .to_string()
        .contains("remote branches cannot be deleted here"));
}

#[test]
fn delete_branch_refuses_a_dash_prefixed_name() {
    let repository_path = scratch_repository("dash");
    let error = open(&repository_path)
        .delete_branch("-D")
        .expect_err("a dash prefixed name is not a branch name");
    assert!(matches!(error, GitError::InvalidBranchName { .. }));
}
