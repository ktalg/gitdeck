use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gitdeck_git::{CliGit, CommitRequest, GitBackend, GitError};
use gitdeck_testkit::{
    copy_directory, fixture_git_command, fixture_repo, scratch_directory, FixtureRepo,
};

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/hunk-staging";
const NOTES_PATH: &str = "notes.txt";
const LINE_COUNT: usize = 20;
const FIRST_CHANGED_LINE: usize = 2;
const SECOND_CHANGED_LINE: usize = 19;
const STAGED_CHANGED_LINE: usize = 3;
const MIDDLE_CHANGED_LINE: usize = 10;
const LINE_BREAK: &str = "\n";

fn scratch_repository(name: &str) -> PathBuf {
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    copy_directory(&fixture_repo(FixtureRepo::Empty), &target)
        .expect("the empty fixture repository is copied");
    run_git(&target, &["config", "core.autocrlf", "false"]);
    target
}

fn git_command(repository_path: &Path, arguments: &[&str]) -> Command {
    let mut command = fixture_git_command(repository_path);
    command.args(arguments);
    command
}

fn run_git_bytes(repository_path: &Path, arguments: &[&str]) -> Vec<u8> {
    let output = git_command(repository_path, arguments)
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn run_git(repository_path: &Path, arguments: &[&str]) -> String {
    String::from_utf8_lossy(&run_git_bytes(repository_path, arguments)).to_string()
}

fn staged_paths(repository_path: &Path) -> String {
    run_git(repository_path, &["diff", "--cached", "--name-only"])
}

fn write_notes(repository_path: &Path, changed_lines: &[usize]) {
    write_notes_with_line_break(repository_path, changed_lines, LINE_BREAK);
}

fn write_notes_with_line_break(repository_path: &Path, changed_lines: &[usize], line_break: &str) {
    let content: String = (1..=LINE_COUNT)
        .map(|line_number| {
            if changed_lines.contains(&line_number) {
                format!("line {line_number:02} changed{line_break}")
            } else {
                format!("line {line_number:02}{line_break}")
            }
        })
        .collect();
    fs::write(repository_path.join(NOTES_PATH), content)
        .unwrap_or_else(|error| panic!("cannot write {NOTES_PATH}: {error}"));
}

fn two_hunk_repository(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    write_notes(&repository_path, &[]);
    run_git(&repository_path, &["add", "--", NOTES_PATH]);
    run_git(&repository_path, &["commit", "-q", "-m", "base notes"]);
    write_notes(&repository_path, &[FIRST_CHANGED_LINE, SECOND_CHANGED_LINE]);
    repository_path
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

fn count_carriage_returns(bytes: &[u8]) -> usize {
    bytes.iter().filter(|byte| **byte == b'\r').count()
}

#[test]
fn stage_hunks_stages_only_the_selected_hunk() {
    let repository_path = two_hunk_repository("selected-hunk");
    let git = CliGit::open(&repository_path).expect("repository opens");
    git.stage_hunks(NOTES_PATH, &[0]).expect("hunk zero stages");

    let staged = run_git(&repository_path, &["diff", "--cached", "--", NOTES_PATH]);
    assert!(
        staged.contains("+line 02 changed\n"),
        "the first hunk reaches the index: {staged}"
    );
    assert!(
        !staged.contains("line 19 changed"),
        "the second hunk stays out of the index: {staged}"
    );

    let unstaged = run_git(&repository_path, &["diff", "--", NOTES_PATH]);
    assert!(
        unstaged.contains("+line 19 changed\n"),
        "the second hunk stays in the work tree: {unstaged}"
    );
    assert!(
        !unstaged.contains("line 02 changed"),
        "the staged hunk leaves the work tree diff: {unstaged}"
    );

    let status = run_git(
        &repository_path,
        &["status", "--porcelain=v2", "--", NOTES_PATH],
    );
    assert!(
        status.starts_with("1 MM "),
        "the file is both staged and modified: {status:?}"
    );
}

#[test]
fn committing_the_selected_hunk_leaves_the_unselected_hunk_in_the_worktree() {
    let repository_path = two_hunk_repository("commit-the-index");
    let git = CliGit::open(&repository_path).expect("repository opens");
    git.commit(hunk_commit_request(
        "stage the first hunk",
        &[(NOTES_PATH, vec![0])],
    ))
    .expect("the selected hunk commits");

    let committed_paths = run_git(
        &repository_path,
        &["show", "--name-only", "--format=", "HEAD"],
    );
    assert_eq!(committed_paths.trim(), NOTES_PATH);

    let committed_content = run_git(&repository_path, &["show", &format!("HEAD:{NOTES_PATH}")]);
    assert!(
        committed_content.contains("line 02 changed"),
        "the staged hunk is in the commit: {committed_content}"
    );
    assert!(
        committed_content.contains("line 19\n"),
        "the unstaged hunk is not in the commit: {committed_content}"
    );

    let remaining = run_git(&repository_path, &["diff", "HEAD", "--", NOTES_PATH]);
    assert!(
        remaining.contains("-line 19\n") && remaining.contains("+line 19 changed\n"),
        "the second hunk survives in the work tree: {remaining}"
    );
    assert!(
        !remaining.contains("line 02"),
        "the committed hunk leaves the work tree diff: {remaining}"
    );
}

#[test]
fn stage_hunks_preserves_crlf_bytes() {
    let repository_path = scratch_repository("crlf-bytes");
    let git = CliGit::open(&repository_path).expect("repository opens");
    write_notes_with_line_break(&repository_path, &[], "\r\n");
    run_git(&repository_path, &["add", "--", NOTES_PATH]);
    run_git(&repository_path, &["commit", "-q", "-m", "base notes"]);
    write_notes_with_line_break(
        &repository_path,
        &[FIRST_CHANGED_LINE, SECOND_CHANGED_LINE],
        "\r\n",
    );

    git.stage_hunks(NOTES_PATH, &[0]).expect("hunk zero stages");

    let index_bytes = run_git_bytes(&repository_path, &["show", &format!(":{NOTES_PATH}")]);
    let index_text = String::from_utf8_lossy(&index_bytes);
    assert!(
        index_text.contains("line 02 changed\r\n"),
        "the staged hunk keeps its carriage returns: {index_text:?}"
    );
    assert!(
        index_text.contains("line 19\r\n"),
        "the unstaged hunk keeps its carriage returns in the index: {index_text:?}"
    );
    assert_eq!(
        count_carriage_returns(&index_bytes),
        LINE_COUNT,
        "every index line still ends with a carriage return"
    );

    let worktree_bytes = fs::read(repository_path.join(NOTES_PATH))
        .unwrap_or_else(|error| panic!("cannot read {NOTES_PATH}: {error}"));
    assert_eq!(
        count_carriage_returns(&worktree_bytes),
        LINE_COUNT,
        "every work tree line still ends with a carriage return"
    );
    assert_eq!(
        String::from_utf8_lossy(&worktree_bytes),
        (1..=LINE_COUNT)
            .map(|line_number| {
                if line_number == FIRST_CHANGED_LINE || line_number == SECOND_CHANGED_LINE {
                    format!("line {line_number:02} changed\r\n")
                } else {
                    format!("line {line_number:02}\r\n")
                }
            })
            .collect::<String>(),
        "the work tree file keeps its own bytes"
    );
}

#[test]
fn stage_hunks_adds_to_a_dirty_index() {
    let repository_path = two_hunk_repository("dirty-index");
    let git = CliGit::open(&repository_path).expect("repository opens");
    fs::write(repository_path.join("other.txt"), "staged other\n")
        .expect("the other file is written");
    run_git(&repository_path, &["add", "--", "other.txt"]);

    git.stage_hunks(NOTES_PATH, &[0])
        .expect("a dirty index accepts the selected hunk");

    let staged_names = staged_paths(&repository_path);
    assert_eq!(
        staged_names.lines().collect::<Vec<_>>(),
        vec!["notes.txt", "other.txt"],
        "the hunk joins the content that was staged before"
    );
}

#[test]
fn stage_hunks_refuses_binary() {
    let repository_path = scratch_repository("binary");
    let git = CliGit::open(&repository_path).expect("repository opens");
    let binary_path = repository_path.join("logo.bin");
    fs::write(&binary_path, b"logo\n\x00\x00\x01\x02\n").expect("the binary file is written");
    run_git(&repository_path, &["add", "--", "logo.bin"]);
    run_git(&repository_path, &["commit", "-q", "-m", "base binary"]);
    fs::write(&binary_path, b"logo\n\x00\x00\x03\x04\n").expect("the binary change is written");

    let error = git
        .stage_hunks("logo.bin", &[0])
        .expect_err("a binary file blocks hunk staging");
    assert!(
        error
            .to_string()
            .contains("binary files must be committed whole"),
        "{error}"
    );
    assert_eq!(staged_paths(&repository_path).trim(), "");
}

#[test]
fn stage_hunks_refuses_rename() {
    let repository_path = scratch_repository("rename");
    let git = CliGit::open(&repository_path).expect("repository opens");
    write_notes(&repository_path, &[]);
    run_git(&repository_path, &["add", "--", NOTES_PATH]);
    run_git(&repository_path, &["commit", "-q", "-m", "base notes"]);
    run_git(&repository_path, &["mv", NOTES_PATH, "moved.txt"]);
    let staged_before = staged_paths(&repository_path);
    assert_eq!(staged_before.trim(), "moved.txt");

    let error = git
        .stage_hunks("moved.txt", &[0])
        .expect_err("a rename blocks hunk staging");
    assert!(
        error
            .to_string()
            .contains("renamed files must be committed whole"),
        "{error}"
    );
    assert_eq!(staged_paths(&repository_path), staged_before);
}

#[test]
fn stage_hunks_refuses_untracked() {
    let repository_path = two_hunk_repository("untracked");
    let git = CliGit::open(&repository_path).expect("repository opens");
    fs::write(repository_path.join("fresh.txt"), "fresh content\n").expect("the fresh file exists");

    let error = git
        .stage_hunks("fresh.txt", &[0])
        .expect_err("an untracked file blocks hunk staging");
    assert!(
        error
            .to_string()
            .contains("untracked files must be committed whole"),
        "{error}"
    );
    assert_eq!(staged_paths(&repository_path).trim(), "");
}

#[test]
fn stage_hunks_refuses_out_of_range_indices() {
    let repository_path = two_hunk_repository("out-of-range");
    let git = CliGit::open(&repository_path).expect("repository opens");

    let error = git
        .stage_hunks(NOTES_PATH, &[5])
        .expect_err("an index past the last hunk is rejected");
    assert!(error.to_string().contains("out of range"), "{error}");

    let error = git
        .stage_hunks(NOTES_PATH, &[])
        .expect_err("an empty selection is rejected");
    assert!(error.to_string().contains("no hunk is selected"), "{error}");

    assert_eq!(staged_paths(&repository_path).trim(), "");
}

#[test]
fn commit_takes_the_worktree_version_of_a_checked_file() {
    let repository_path = scratch_repository("worktree-over-index");
    let git = CliGit::open(&repository_path).expect("repository opens");
    write_notes(&repository_path, &[]);
    run_git(&repository_path, &["add", "--", NOTES_PATH]);
    run_git(&repository_path, &["commit", "-q", "-m", "base notes"]);
    fs::write(repository_path.join(NOTES_PATH), "index version\n").expect("the index version");
    run_git(&repository_path, &["add", "--", NOTES_PATH]);
    fs::write(repository_path.join(NOTES_PATH), "worktree version\n")
        .expect("the work tree version");

    git.commit(commit_request("commit the file", &[NOTES_PATH]))
        .expect("the file commits");

    let committed_content = run_git(&repository_path, &["show", &format!("HEAD:{NOTES_PATH}")]);
    assert_eq!(committed_content, "worktree version\n");
    let worktree_content = fs::read_to_string(repository_path.join(NOTES_PATH))
        .unwrap_or_else(|error| panic!("cannot read {NOTES_PATH}: {error}"));
    assert_eq!(
        worktree_content, "worktree version\n",
        "the work tree keeps its own version"
    );
    let remaining = run_git(&repository_path, &["diff", "HEAD", "--", NOTES_PATH]);
    assert!(
        remaining.trim().is_empty(),
        "the committed version matches the work tree: {remaining}"
    );
    assert_eq!(
        staged_paths(&repository_path).trim(),
        "",
        "the staged version leaves the index instead of staying behind"
    );
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

fn two_hunk_file_repository(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    for path in [NOTES_PATH, "extra.txt"] {
        write_numbered_lines(&repository_path.join(path), &[]);
        run_git(&repository_path, &["add", "--", path]);
    }
    run_git(&repository_path, &["commit", "-q", "-m", "base notes"]);
    for path in [NOTES_PATH, "extra.txt"] {
        write_numbered_lines(
            &repository_path.join(path),
            &[FIRST_CHANGED_LINE, SECOND_CHANGED_LINE],
        );
    }
    repository_path
}

#[test]
fn stage_hunk_selection_checks_the_index_once_for_every_file() {
    let repository_path = two_hunk_file_repository("batch-selection");
    let git = CliGit::open(&repository_path).expect("repository opens");
    git.stage_hunk_selection(&[
        (NOTES_PATH.to_string(), vec![0]),
        (String::from("extra.txt"), vec![1]),
    ])
    .expect("both files stage their selected hunk");

    let staged = run_git(&repository_path, &["diff", "--cached"]);
    assert!(
        staged.contains("+line 02 changed\n"),
        "the first file stages its first hunk: {staged}"
    );
    assert_eq!(
        staged.matches("+line 19 changed\n").count(),
        1,
        "only the second file stages its second hunk: {staged}"
    );
    assert!(
        staged.contains("+line 02 changed\n"),
        "the first file keeps its first hunk: {staged}"
    );
    let staged_names = staged_paths(&repository_path);
    assert_eq!(
        staged_names.lines().collect::<Vec<_>>(),
        vec!["extra.txt", NOTES_PATH],
        "both files reach the index"
    );
}

#[test]
fn stage_hunk_selection_adds_to_a_dirty_index() {
    let repository_path = two_hunk_file_repository("batch-dirty-index");
    let git = CliGit::open(&repository_path).expect("repository opens");
    fs::write(repository_path.join("other.txt"), "staged other\n")
        .expect("the other file is written");
    run_git(&repository_path, &["add", "--", "other.txt"]);

    git.stage_hunk_selection(&[
        (NOTES_PATH.to_string(), vec![0]),
        (String::from("extra.txt"), vec![0]),
    ])
    .expect("a dirty index accepts the selection");

    let staged_names = staged_paths(&repository_path);
    assert_eq!(
        staged_names.lines().collect::<Vec<_>>(),
        vec!["extra.txt", "notes.txt", "other.txt"],
        "the hunks join the content that was staged before"
    );
}

#[test]
fn stage_hunk_selection_names_the_files_left_in_the_index() {
    let repository_path = two_hunk_file_repository("batch-partial");
    let git = CliGit::open(&repository_path).expect("repository opens");
    fs::write(repository_path.join("fresh.txt"), "fresh content\n").expect("the fresh file exists");

    let error = git
        .stage_hunk_selection(&[
            (NOTES_PATH.to_string(), vec![0]),
            (String::from("fresh.txt"), vec![0]),
        ])
        .expect_err("the untracked file stops the batch");
    let message = error.to_string();
    assert!(
        message.contains("untracked files must be committed whole"),
        "{message}"
    );
    assert!(
        message.contains("the hunks of 1 file(s) stay staged in the index"),
        "the error names the staged hunks it left behind: {message}"
    );
    assert_eq!(staged_paths(&repository_path).trim(), NOTES_PATH);
}

fn partially_staged_repository(
    name: &str,
    staged_lines: &[usize],
    unstaged_lines: &[usize],
) -> PathBuf {
    let repository_path = scratch_repository(name);
    write_notes(&repository_path, &[]);
    run_git(&repository_path, &["add", "--", NOTES_PATH]);
    run_git(&repository_path, &["commit", "-q", "-m", "base notes"]);
    write_notes(&repository_path, staged_lines);
    run_git(&repository_path, &["add", "--", NOTES_PATH]);
    let worktree_lines: Vec<usize> = staged_lines.iter().chain(unstaged_lines).copied().collect();
    write_notes(&repository_path, &worktree_lines);
    repository_path
}

fn hunk_header_count(diff: &str) -> usize {
    diff.lines().filter(|line| line.starts_with("@@")).count()
}

#[test]
fn hunk_staging_works_on_a_file_with_staged_changes() {
    let repository_path = partially_staged_repository(
        "staged-and-modified",
        &[STAGED_CHANGED_LINE],
        &[MIDDLE_CHANGED_LINE, SECOND_CHANGED_LINE],
    );
    let git = CliGit::open(&repository_path).expect("repository opens");
    let status = run_git(
        &repository_path,
        &["status", "--porcelain=v2", "--", NOTES_PATH],
    );
    assert!(
        status.starts_with("1 MM "),
        "the file is staged and modified: {status:?}"
    );

    git.commit(hunk_commit_request(
        "commit the selected hunk",
        &[(NOTES_PATH, vec![0])],
    ))
    .expect("the selected hunk commits although the file holds staged changes");

    let committed_paths = run_git(
        &repository_path,
        &["show", "--name-only", "--format=", "HEAD"],
    );
    assert_eq!(committed_paths.trim(), NOTES_PATH);
    let committed_content = run_git(&repository_path, &["show", &format!("HEAD:{NOTES_PATH}")]);
    assert!(
        committed_content.contains("line 03 changed"),
        "the staged part of the file reaches the commit: {committed_content}"
    );
    assert!(
        committed_content.contains("line 10 changed"),
        "the selected hunk reaches the commit: {committed_content}"
    );
    assert!(
        !committed_content.contains("line 19 changed"),
        "the unchecked hunk stays out of the commit: {committed_content}"
    );
    let remaining = run_git(&repository_path, &["diff", "--", NOTES_PATH]);
    assert!(
        remaining.contains("+line 19 changed"),
        "the unchecked hunk stays in the work tree: {remaining}"
    );
    assert!(
        !remaining.contains("line 10 changed"),
        "the committed hunk leaves the work tree diff: {remaining}"
    );
    assert_eq!(
        run_git(&repository_path, &["diff", "--cached"]).trim(),
        "",
        "the index matches the new head after the commit"
    );
}

#[test]
fn hunk_staging_patch_is_relative_to_the_index() {
    let repository_path = partially_staged_repository(
        "index-relative-patch",
        &[STAGED_CHANGED_LINE],
        &[MIDDLE_CHANGED_LINE],
    );
    let git = CliGit::open(&repository_path).expect("repository opens");
    let staged_before = run_git(&repository_path, &["diff", "--cached", "--", NOTES_PATH]);
    assert!(
        staged_before.contains("+line 03 changed"),
        "the index holds a change the work tree diff does not show: {staged_before}"
    );
    let worktree_diff = run_git(&repository_path, &["diff", "--", NOTES_PATH]);
    assert_eq!(
        hunk_header_count(&worktree_diff),
        1,
        "the work tree diff holds the unstaged hunk only: {worktree_diff}"
    );
    assert!(
        !worktree_diff.contains("line 03 changed"),
        "the staged change stays out of the work tree diff: {worktree_diff}"
    );

    git.commit(hunk_commit_request(
        "commit the unstaged hunk",
        &[(NOTES_PATH, vec![0])],
    ))
    .expect("a patch taken from the work tree diff applies to the index");

    let committed_content = run_git(&repository_path, &["show", &format!("HEAD:{NOTES_PATH}")]);
    assert!(
        committed_content.contains("line 03 changed"),
        "the staged change survives the commit: {committed_content}"
    );
    assert!(
        committed_content.contains("line 10 changed"),
        "the selected hunk reaches the commit: {committed_content}"
    );
}

#[test]
fn hunk_staging_failure_reports_a_hint() {
    let error = GitError::HunkPatchFailed {
        path: String::from(NOTES_PATH),
        message: String::from(
            "git apply --cached --whitespace=nowarn - failed in /repo: error: patch failed: notes.txt:1",
        ),
    };
    let message = error.to_string();
    assert!(message.contains(NOTES_PATH), "{message}");
    assert!(
        message.contains("error: patch failed"),
        "the git reason stays visible: {message}"
    );
    assert!(
        message
            .contains("next step: cancel the staged changes of the file and select the hunk again"),
        "the error names the next step: {message}"
    );
}
