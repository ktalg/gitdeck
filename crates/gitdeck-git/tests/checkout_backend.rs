use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_core::{LogFilter, LogQuery};
use gitdeck_git::{CliGit, GitBackend, GitError};
use gitdeck_testkit::{
    copy_directory, fixture_git_command, fixture_repo, scratch_directory, FixtureRepo,
};

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/checkout-backend";
const CURRENT_BRANCH: &str = "main";
const OTHER_BRANCH: &str = "feature/other";
const UNKNOWN_BRANCH: &str = "no-such-branch";
const APP_PATH: &str = "src/app.rs";
const CLEAN_TREE_SUBJECT: &str = "clean work tree";
const OTHER_BRANCH_SUBJECT: &str = "call run on the other branch";
const LOCALLY_EDITED_APP: &str = "fn main() {\n    run_locally();\n}\n";
const DIRTY_CHECKOUT_STDERR_KEY: &str = "would be overwritten by checkout";
const UNKNOWN_BRANCH_STDERR_KEY: &str = "did not match any file(s) known to git";

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

fn commit_requested_paths(repository_path: &Path, message: &str) {
    run_git(repository_path, &["add", "--all"]);
    run_git(repository_path, &["commit", "-q", "-m", message]);
}

fn write_file(repository_path: &Path, relative_path: &str, contents: &str) {
    let path = repository_path.join(relative_path);
    fs::write(&path, contents)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
}

fn repository_with_a_second_branch(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    commit_requested_paths(&repository_path, CLEAN_TREE_SUBJECT);
    run_git(&repository_path, &["checkout", "-q", "-b", OTHER_BRANCH]);
    write_file(
        &repository_path,
        APP_PATH,
        "fn main() {\n    run_on_the_other_branch();\n}\n",
    );
    run_git(
        &repository_path,
        &["commit", "-q", "-a", "-m", OTHER_BRANCH_SUBJECT],
    );
    run_git(&repository_path, &["checkout", "-q", CURRENT_BRANCH]);
    repository_path
}

fn checked_out_branch(git: &CliGit) -> Option<String> {
    git.head_info().expect("head info read").branch
}

fn first_commit_subject(git: &CliGit) -> String {
    let commits = git
        .log(
            LogQuery {
                max_commits: 1,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("log read");
    commits.first().expect("a commit exists").subject.clone()
}

fn command_failed_message(error: GitError) -> String {
    match error {
        GitError::CommandFailed {
            arguments, message, ..
        } => {
            assert!(!message.is_empty(), "the failure carries git stderr");
            assert!(
                arguments.starts_with("checkout "),
                "the failure names the checkout command, got {arguments:?}"
            );
            message
        }
        other => panic!("expected CommandFailed, got {other:?}"),
    }
}

#[test]
fn checkout_branch_switches_the_head() {
    let repository_path = repository_with_a_second_branch("switches-head");
    let git = CliGit::open(&repository_path).expect("repository opens");
    assert_eq!(checked_out_branch(&git).as_deref(), Some(CURRENT_BRANCH));
    assert_eq!(first_commit_subject(&git), CLEAN_TREE_SUBJECT);

    git.checkout_branch(OTHER_BRANCH)
        .expect("the checkout succeeds");

    assert_eq!(checked_out_branch(&git).as_deref(), Some(OTHER_BRANCH));
    assert_eq!(first_commit_subject(&git), OTHER_BRANCH_SUBJECT);
    assert_eq!(
        run_git(&repository_path, &["branch", "--show-current"]).trim(),
        OTHER_BRANCH
    );
}

#[test]
fn checkout_branch_reports_dirty_worktree_failure() {
    let repository_path = repository_with_a_second_branch("dirty-worktree");
    let git = CliGit::open(&repository_path).expect("repository opens");
    write_file(&repository_path, APP_PATH, LOCALLY_EDITED_APP);

    let error = git
        .checkout_branch(OTHER_BRANCH)
        .expect_err("the dirty work tree blocks the checkout");

    let message = command_failed_message(error);
    assert!(
        message.contains(DIRTY_CHECKOUT_STDERR_KEY),
        "the panel error keeps the git text, got {message:?}"
    );
    assert_eq!(checked_out_branch(&git).as_deref(), Some(CURRENT_BRANCH));
    assert_eq!(
        fs::read_to_string(repository_path.join(APP_PATH)).expect("the edited file is readable"),
        LOCALLY_EDITED_APP
    );
}

#[test]
fn checkout_unknown_branch_reports_an_error() {
    let repository_path = repository_with_a_second_branch("unknown-branch");
    let git = CliGit::open(&repository_path).expect("repository opens");

    let error = git
        .checkout_branch(UNKNOWN_BRANCH)
        .expect_err("an unknown branch cannot be checked out");

    let message = command_failed_message(error);
    assert!(
        message.contains(UNKNOWN_BRANCH_STDERR_KEY),
        "the panel error keeps the git text, got {message:?}"
    );
    assert_eq!(checked_out_branch(&git).as_deref(), Some(CURRENT_BRANCH));
}
