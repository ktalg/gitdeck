use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_git::{CliGit, GitBackend, GitError};
use gitdeck_testkit::{
    copy_directory, fixture_git_command, fixture_repo, scratch_directory, FixtureRepo,
};

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/create-branch-backend";
const CURRENT_BRANCH: &str = "main";
const START_POINT_BRANCH: &str = "feature/start";
const NEW_BRANCH: &str = "feature/created";
const START_POINT_SUBJECT: &str = "work on the start branch";
const BASE_SUBJECT: &str = "clean work tree";
const APP_PATH: &str = "src/app.rs";
const EXISTING_BRANCH_STDERR_KEY: &str = "already exists";

fn scratch_repository(name: &str) -> PathBuf {
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    copy_directory(&fixture_repo(FixtureRepo::Dirty), &target)
        .expect("the fixture repository is copied");
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

fn commit_everything(repository_path: &Path, message: &str) {
    run_git(repository_path, &["add", "--all"]);
    run_git(repository_path, &["commit", "-q", "-m", message]);
}

fn repository_with_a_start_branch(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    commit_everything(&repository_path, BASE_SUBJECT);
    run_git(
        &repository_path,
        &["checkout", "-q", "-b", START_POINT_BRANCH],
    );
    fs::write(
        repository_path.join(APP_PATH),
        "fn main() {\n    run_on_the_start_branch();\n}\n",
    )
    .expect("the start branch change is written");
    commit_everything(&repository_path, START_POINT_SUBJECT);
    run_git(&repository_path, &["checkout", "-q", CURRENT_BRANCH]);
    repository_path
}

fn head_commit_id(repository_path: &Path) -> String {
    run_git(repository_path, &["rev-parse", "HEAD"])
        .trim()
        .to_string()
}

fn current_branch(repository_path: &Path) -> String {
    run_git(repository_path, &["branch", "--show-current"])
        .trim()
        .to_string()
}

#[test]
fn create_branch_switches_to_the_new_branch() {
    let repository_path = repository_with_a_start_branch("switches");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let head_before = head_commit_id(&repository_path);

    git.create_branch(NEW_BRANCH, None)
        .expect("the branch is created");

    assert_eq!(current_branch(&repository_path), NEW_BRANCH);
    assert_eq!(
        git.head_info().expect("head info read").branch.as_deref(),
        Some(NEW_BRANCH)
    );
    assert_eq!(
        head_commit_id(&repository_path),
        head_before,
        "the new branch starts on the HEAD commit"
    );
}

#[test]
fn create_branch_from_a_start_point() {
    let repository_path = repository_with_a_start_branch("start-point");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let start_point_head = run_git(
        &repository_path,
        &["rev-parse", &format!("refs/heads/{START_POINT_BRANCH}")],
    )
    .trim()
    .to_string();

    git.create_branch(NEW_BRANCH, Some(START_POINT_BRANCH))
        .expect("the branch is created from the start point");

    assert_eq!(current_branch(&repository_path), NEW_BRANCH);
    assert_eq!(head_commit_id(&repository_path), start_point_head);
    assert_eq!(
        run_git(&repository_path, &["log", "-1", "--format=%s"]).trim(),
        START_POINT_SUBJECT
    );
    assert_eq!(
        current_branch(&repository_path),
        NEW_BRANCH,
        "the start point branch itself stays untouched"
    );
    assert_eq!(
        run_git(
            &repository_path,
            &["log", "-1", "--format=%s", START_POINT_BRANCH]
        )
        .trim(),
        START_POINT_SUBJECT
    );
}

#[test]
fn create_branch_reports_invalid_names() {
    for name in ["bad name", "a..b", "-x", "", "with~tilde", "trailing/"] {
        let repository_path = repository_with_a_start_branch("invalid-name");
        let git = CliGit::open(&repository_path).expect("repository opens");
        let head_before = head_commit_id(&repository_path);
        let branch_before = current_branch(&repository_path);

        let error = git
            .create_branch(name, None)
            .expect_err("the invalid name is rejected");

        match error {
            GitError::InvalidBranchName { name: reported } => assert_eq!(reported, name),
            other => panic!("expected InvalidBranchName for {name:?}, got {other:?}"),
        }
        assert_eq!(
            current_branch(&repository_path),
            branch_before,
            "the rejected name {name:?} leaves the head branch alone"
        );
        assert_eq!(
            head_commit_id(&repository_path),
            head_before,
            "the rejected name {name:?} leaves the head commit alone"
        );
        if !name.is_empty() {
            assert!(
                !run_git(&repository_path, &["branch", "--list"]).contains(name.trim()),
                "the rejected name {name:?} creates no branch"
            );
        }
    }
}

#[test]
fn create_branch_reports_an_existing_branch() {
    let repository_path = repository_with_a_start_branch("existing");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let branch_before = current_branch(&repository_path);

    let error = git
        .create_branch(START_POINT_BRANCH, None)
        .expect_err("an existing branch cannot be created again");

    match error {
        GitError::CommandFailed {
            arguments, message, ..
        } => {
            assert!(
                arguments.contains(START_POINT_BRANCH),
                "the failure names the rejected branch, got {arguments:?}"
            );
            assert!(
                message.contains(EXISTING_BRANCH_STDERR_KEY),
                "the failure keeps the git text, got {message:?}"
            );
        }
        other => panic!("expected CommandFailed, got {other:?}"),
    }
    assert_eq!(
        current_branch(&repository_path),
        branch_before,
        "the failed creation leaves the head branch alone"
    );
}
