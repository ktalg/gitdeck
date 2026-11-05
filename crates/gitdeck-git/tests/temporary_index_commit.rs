use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_git::{CliGit, CommitRequest, GitBackend, GitError};
use gitdeck_testkit::{
    copy_directory, fixture_git_command, fixture_repo, scratch_directory, FixtureRepo,
};

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/temporary-index-commit";
const INDEX_FILE_NAME_PREFIX: &str = "gitdeck-commit-index-";
const LINE_COUNT: usize = 20;
const FIRST_CHANGED_LINE: usize = 2;
const SECOND_CHANGED_LINE: usize = 19;

fn scratch_repository(name: &str) -> PathBuf {
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    copy_directory(&fixture_repo(FixtureRepo::Empty), &target)
        .expect("the empty fixture repository is copied");
    run_git(&target, &["config", "core.autocrlf", "false"]);
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

fn git_directory(repository_path: &Path) -> PathBuf {
    PathBuf::from(run_git(repository_path, &["rev-parse", "--absolute-git-dir"]).trim())
}

fn temporary_index_files(repository_path: &Path) -> Vec<PathBuf> {
    let entries = fs::read_dir(git_directory(repository_path))
        .unwrap_or_else(|error| panic!("cannot list the git directory: {error}"));
    entries
        .map(|entry| entry.expect("the git directory entry is readable").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(INDEX_FILE_NAME_PREFIX))
        })
        .collect()
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

fn hunk_commit_request(message: &str, hunks: &[(&str, Vec<usize>)]) -> CommitRequest {
    CommitRequest {
        message: message.to_string(),
        amend: false,
        sign_off: false,
        paths: hunks.iter().map(|(path, _)| path.to_string()).collect(),
        hunks: hunks
            .iter()
            .map(|(path, hunk_indices)| (path.to_string(), hunk_indices.clone()))
            .collect(),
    }
}

fn two_file_repository(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    fs::write(repository_path.join("a.txt"), "a base\n").expect("a.txt is written");
    fs::write(repository_path.join("b.txt"), "b base\n").expect("b.txt is written");
    run_git(&repository_path, &["add", "--", "a.txt", "b.txt"]);
    run_git(&repository_path, &["commit", "-q", "-m", "base files"]);
    fs::write(repository_path.join("a.txt"), "a changed\n").expect("a.txt changes");
    fs::write(repository_path.join("b.txt"), "b changed\n").expect("b.txt changes");
    run_git(&repository_path, &["add", "--", "a.txt", "b.txt"]);
    repository_path
}

fn write_numbered_lines(path: &Path, changed_lines: &[usize]) {
    let content: String = (1..=LINE_COUNT)
        .map(|line_number| {
            if changed_lines.contains(&line_number) {
                format!("line {line_number:02} changed\n")
            } else {
                format!("line {line_number:02}\n")
            }
        })
        .collect();
    fs::write(path, content)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
}

fn two_hunk_repository(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    for path in ["notes.txt", "extra.txt"] {
        write_numbered_lines(&repository_path.join(path), &[]);
        run_git(&repository_path, &["add", "--", path]);
    }
    run_git(&repository_path, &["commit", "-q", "-m", "base notes"]);
    write_numbered_lines(
        &repository_path.join("notes.txt"),
        &[FIRST_CHANGED_LINE, SECOND_CHANGED_LINE],
    );
    repository_path
}

fn staged_paths(repository_path: &Path) -> String {
    run_git(repository_path, &["diff", "--cached", "--name-only"])
}

#[test]
fn commit_skips_a_pre_staged_but_unchecked_path() {
    let repository_path = two_file_repository("unchecked-path");
    let git = CliGit::open(&repository_path).expect("repository opens");
    assert_eq!(staged_paths(&repository_path).trim(), "a.txt\nb.txt");

    git.commit(commit_request("commit a only", &["a.txt"]))
        .expect("the checked file commits");

    let committed = run_git(
        &repository_path,
        &["show", "--name-only", "--format=", "HEAD"],
    );
    assert_eq!(
        committed.trim(),
        "a.txt",
        "the unchecked pre-staged path stays out of the commit"
    );
    let committed_content = run_git(&repository_path, &["show", "HEAD:a.txt"]);
    assert_eq!(committed_content, "a changed\n");
}

#[test]
fn commit_keeps_the_staged_state_of_unchecked_paths() {
    let repository_path = two_file_repository("staged-state");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let staged_before = run_git(&repository_path, &["diff", "--cached", "--", "b.txt"]);

    git.commit(commit_request("commit a only", &["a.txt"]))
        .expect("the checked file commits");

    assert_eq!(
        staged_paths(&repository_path).trim(),
        "b.txt",
        "the unchecked path stays staged"
    );
    assert_eq!(
        run_git(&repository_path, &["diff", "--cached", "--", "b.txt"]),
        staged_before,
        "the staged content of the unchecked path is untouched"
    );
    assert_eq!(
        run_git(&repository_path, &["diff", "--cached", "--", "a.txt"]).trim(),
        "",
        "the committed path leaves the index"
    );
}

#[test]
fn hunk_staging_works_with_a_dirty_index() {
    let repository_path = two_hunk_repository("hunk-dirty-index");
    let git = CliGit::open(&repository_path).expect("repository opens");
    fs::write(repository_path.join("extra.txt"), "extra changed\n").expect("extra.txt changes");
    run_git(&repository_path, &["add", "--", "extra.txt"]);

    git.commit(hunk_commit_request(
        "commit the selected hunk",
        &[("notes.txt", vec![0])],
    ))
    .expect("the selected hunk commits although the index is dirty");

    let committed_content = run_git(&repository_path, &["show", "HEAD:notes.txt"]);
    assert!(
        committed_content.contains("line 02 changed"),
        "the selected hunk reaches the commit: {committed_content}"
    );
    assert!(
        !committed_content.contains("line 19 changed"),
        "the unchecked hunk stays out of the commit: {committed_content}"
    );
    assert_eq!(
        staged_paths(&repository_path).trim(),
        "extra.txt",
        "the index keeps only the content that was staged before"
    );
}

#[test]
fn hunk_staging_leaves_the_real_index_untouched_for_other_paths() {
    let repository_path = two_hunk_repository("hunk-other-index-entries");
    let git = CliGit::open(&repository_path).expect("repository opens");
    fs::write(repository_path.join("extra.txt"), "extra changed\n").expect("extra.txt changes");
    run_git(&repository_path, &["add", "--", "extra.txt"]);
    let other_entries_before = run_git(&repository_path, &["ls-files", "-s", "--", "extra.txt"]);
    let staged_diff_before = run_git(&repository_path, &["diff", "--cached", "--", "extra.txt"]);

    git.commit(hunk_commit_request(
        "commit the selected hunk",
        &[("notes.txt", vec![0])],
    ))
    .expect("the selected hunk commits");

    assert_eq!(
        run_git(&repository_path, &["ls-files", "-s", "--", "extra.txt"]),
        other_entries_before,
        "the index entry of the other path keeps its tracked blob"
    );
    assert_eq!(
        run_git(&repository_path, &["diff", "--cached", "--", "extra.txt"]),
        staged_diff_before,
        "the staged content of the other paths is untouched"
    );
}

#[test]
fn amend_from_a_temp_index_replaces_the_head() {
    let repository_path = two_file_repository("amend");
    run_git(&repository_path, &["commit", "-q", "-m", "checkpoint"]);
    fs::write(repository_path.join("b.txt"), "b worktree only\n").expect("b.txt changes");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let commit_count_before = run_git(&repository_path, &["rev-list", "--count", "HEAD"]);
    let parent_before = run_git(&repository_path, &["rev-parse", "HEAD^{tree}"]);

    let request = CommitRequest {
        amend: true,
        ..commit_request("reworded head", &[])
    };
    git.commit(request).expect("the amend succeeds");

    assert_eq!(
        run_git(&repository_path, &["rev-list", "--count", "HEAD"]),
        commit_count_before,
        "the amend replaces the head instead of adding a commit"
    );
    assert_eq!(
        run_git(&repository_path, &["log", "-1", "--format=%s"]).trim(),
        "reworded head"
    );
    assert_eq!(
        run_git(&repository_path, &["rev-parse", "HEAD^{tree}"]),
        parent_before,
        "the amend keeps the tree it started from"
    );
    assert_eq!(
        fs::read_to_string(repository_path.join("b.txt")).expect("b.txt is readable"),
        "b worktree only\n",
        "the work tree content is untouched"
    );
    assert_eq!(
        staged_paths(&repository_path).trim(),
        "",
        "the amend stages nothing"
    );
}

#[test]
fn commit_leaves_no_temporary_index_file_behind() {
    let repository_path = two_file_repository("temporary-index-cleanup");
    let git = CliGit::open(&repository_path).expect("repository opens");
    git.commit(commit_request("commit a only", &["a.txt"]))
        .expect("the checked file commits");
    assert!(
        temporary_index_files(&repository_path).is_empty(),
        "a successful commit removes its temporary index"
    );

    let failed = git.commit(hunk_commit_request(
        "commit a missing hunk",
        &[("b.txt", vec![5])],
    ));
    assert!(failed.is_err(), "an out of range hunk index fails");
    assert!(
        temporary_index_files(&repository_path).is_empty(),
        "a failed commit removes its temporary index"
    );
}

#[test]
fn sign_off_is_written_through_the_temporary_index() {
    let repository_path = two_file_repository("sign-off");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let request = CommitRequest {
        sign_off: true,
        ..commit_request("signed commit", &["a.txt"])
    };
    git.commit(request).expect("the signed commit succeeds");

    let message = run_git(&repository_path, &["log", "-1", "--format=%B"]);
    assert!(
        message.contains("Signed-off-by: gitdeck-fixture <fixture@gitdeck.invalid>"),
        "the trailer is missing: {message:?}"
    );
    assert_eq!(
        run_git(
            &repository_path,
            &["show", "--name-only", "--format=", "HEAD"]
        )
        .trim(),
        "a.txt",
        "the signed commit holds only the checked path"
    );
}

#[test]
fn a_commit_failure_leaves_the_repository_unchanged() {
    let repository_path = two_file_repository("failed-commit");
    run_git(&repository_path, &["commit", "-q", "-m", "checkpoint"]);
    let git = CliGit::open(&repository_path).expect("repository opens");
    let head_before = run_git(&repository_path, &["rev-parse", "HEAD"]);
    let index_entries_before = run_git(&repository_path, &["ls-files", "-s"]);

    let error = git
        .commit(commit_request("commit nothing", &["a.txt"]))
        .expect_err("a commit without staged content fails");
    assert!(
        matches!(error, GitError::CommandFailed { .. }),
        "the failure comes from git: {error}"
    );
    assert_eq!(
        run_git(&repository_path, &["rev-parse", "HEAD"]),
        head_before,
        "the failed commit leaves the head alone"
    );
    assert_eq!(
        run_git(&repository_path, &["ls-files", "-s"]),
        index_entries_before,
        "the failed commit leaves the index alone"
    );
    assert!(
        temporary_index_files(&repository_path).is_empty(),
        "the failed commit removes its temporary index"
    );
}
