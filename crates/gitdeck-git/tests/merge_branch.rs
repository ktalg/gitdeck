use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_git::{CliGit, CommitRequest, ConflictResolution, GitBackend, GitError, MergeOutcome};
use gitdeck_testkit::fixture_git_command;
use gitdeck_testkit::repository_root;

const SCRATCH_DIRECTORY: &str = "artifacts/tmp/merge-branch";
const MAIN_BRANCH: &str = "main";
const FEATURE_BRANCH: &str = "feature";
const SHARED_PATH: &str = "src/shared.txt";
const MAIN_ONLY_PATH: &str = "src/main.txt";
const FEATURE_ONLY_PATH: &str = "src/feature.txt";
const BASE_CONTENT: &str = "first line\nshared line\nlast line\n";
const OUR_CONTENT: &str = "first line\nmain line\nlast line\n";
const THEIR_CONTENT: &str = "first line\nfeature line\nlast line\n";
const BASE_SUBJECT: &str = "base commit";
const MAIN_SUBJECT: &str = "main branch change";
const FEATURE_SUBJECT: &str = "feature branch change";
const MERGE_MESSAGE: &str = "merge the feature branch";
const UNMERGED_STATUS_PREFIX: &str = "UU ";
const SECOND_SHARED_PATH: &str = "src/second.txt";
const CLEAN_PATH: &str = "src/clean.txt";
const SECOND_BASE_CONTENT: &str = "alpha\nshared line\nomega\n";
const SECOND_OUR_CONTENT: &str = "alpha\nmain line\nomega\n";
const SECOND_THEIR_CONTENT: &str = "alpha\nfeature line\nomega\n";
const CLEAN_BASE_CONTENT: &str = "clean base\n";
const CLEAN_THEIR_CONTENT: &str = "clean feature\n";
const PLAIN_FIRST_PATH: &str = "src/plain-first.txt";
const PLAIN_SECOND_PATH: &str = "src/plain-second.txt";

fn run_git(working_directory: &Path, arguments: &[&str]) -> String {
    let output = fixture_git_command(working_directory)
        .args(["-c", "init.defaultBranch=main"])
        .args(arguments)
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
        &["config", "user.name", "gitdeck-merge-test"],
    );
    run_git(
        &repository_path,
        &["config", "user.email", "merge@gitdeck.invalid"],
    );
    write_file(&repository_path, SHARED_PATH, BASE_CONTENT);
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", BASE_SUBJECT]);
    repository_path
}

fn repository_with_a_branch_ahead(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    run_git(&repository_path, &["switch", "-q", "-c", FEATURE_BRANCH]);
    write_file(&repository_path, FEATURE_ONLY_PATH, "feature only\n");
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", FEATURE_SUBJECT]);
    run_git(&repository_path, &["switch", "-q", MAIN_BRANCH]);
    repository_path
}

fn diverged_repository(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    run_git(&repository_path, &["switch", "-q", "-c", FEATURE_BRANCH]);
    write_file(&repository_path, FEATURE_ONLY_PATH, "feature only\n");
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", FEATURE_SUBJECT]);
    run_git(&repository_path, &["switch", "-q", MAIN_BRANCH]);
    write_file(&repository_path, MAIN_ONLY_PATH, "main only\n");
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", MAIN_SUBJECT]);
    repository_path
}

fn conflicting_repository(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    run_git(&repository_path, &["switch", "-q", "-c", FEATURE_BRANCH]);
    write_file(&repository_path, SHARED_PATH, THEIR_CONTENT);
    run_git(
        &repository_path,
        &["commit", "-q", "-a", "-m", FEATURE_SUBJECT],
    );
    run_git(&repository_path, &["switch", "-q", MAIN_BRANCH]);
    write_file(&repository_path, SHARED_PATH, OUR_CONTENT);
    run_git(
        &repository_path,
        &["commit", "-q", "-a", "-m", MAIN_SUBJECT],
    );
    repository_path
}

fn two_conflict_merge_repository(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    write_file(&repository_path, SECOND_SHARED_PATH, SECOND_BASE_CONTENT);
    write_file(&repository_path, CLEAN_PATH, CLEAN_BASE_CONTENT);
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", BASE_SUBJECT]);
    run_git(&repository_path, &["switch", "-q", "-c", FEATURE_BRANCH]);
    write_file(&repository_path, SHARED_PATH, THEIR_CONTENT);
    write_file(&repository_path, SECOND_SHARED_PATH, SECOND_THEIR_CONTENT);
    write_file(&repository_path, CLEAN_PATH, CLEAN_THEIR_CONTENT);
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", FEATURE_SUBJECT]);
    run_git(&repository_path, &["switch", "-q", MAIN_BRANCH]);
    write_file(&repository_path, SHARED_PATH, OUR_CONTENT);
    write_file(&repository_path, SECOND_SHARED_PATH, SECOND_OUR_CONTENT);
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", MAIN_SUBJECT]);
    repository_path
}

fn resolved_two_conflict_merge(name: &str) -> (PathBuf, CliGit) {
    let repository_path = two_conflict_merge_repository(name);
    let git = open(&repository_path);
    git.merge_branch(FEATURE_BRANCH)
        .expect("the merge reports its conflicts");
    git.resolve_conflict(SHARED_PATH, ConflictResolution::Theirs)
        .expect("the first conflict resolves to their side");
    git.resolve_conflict(SECOND_SHARED_PATH, ConflictResolution::Ours)
        .expect("the second conflict resolves to our side");
    (repository_path, git)
}

fn commit_request_with_paths(message: &str, paths: &[&str]) -> CommitRequest {
    CommitRequest {
        message: String::from(message),
        amend: false,
        sign_off: false,
        paths: paths.iter().map(|path| path.to_string()).collect(),
        hunks: Vec::new(),
    }
}

fn committed_file(repository_path: &Path, path: &str) -> String {
    run_git(repository_path, &["show", &format!("HEAD:{path}")])
}

fn open(repository_path: &Path) -> CliGit {
    CliGit::open(repository_path).expect("the scratch repository opens")
}

fn status_porcelain(repository_path: &Path) -> String {
    run_git(repository_path, &["status", "--porcelain"])
}

fn parent_count(repository_path: &Path) -> usize {
    run_git(repository_path, &["log", "-1", "--format=%P"])
        .split_whitespace()
        .count()
}

#[test]
fn merge_branch_fast_forwards_when_possible() {
    let repository_path = repository_with_a_branch_ahead("fast-forward");
    let git = open(&repository_path);
    let feature_tip = run_git(&repository_path, &["rev-parse", FEATURE_BRANCH])
        .trim()
        .to_string();
    let outcome = git
        .merge_branch(FEATURE_BRANCH)
        .expect("a fast-forwardable merge succeeds");
    assert_eq!(
        outcome,
        MergeOutcome::FastForward {
            commit_id: feature_tip.clone()
        }
    );
    assert_eq!(
        run_git(&repository_path, &["rev-parse", "HEAD"]).trim(),
        feature_tip,
        "the head moves to the merged branch tip"
    );
    assert_eq!(
        parent_count(&repository_path),
        1,
        "no merge commit is created"
    );
    assert!(status_porcelain(&repository_path).trim().is_empty());
}

#[test]
fn merge_branch_creates_a_merge_commit_when_diverged() {
    let repository_path = diverged_repository("diverged");
    let git = open(&repository_path);
    let outcome = git
        .merge_branch(FEATURE_BRANCH)
        .expect("a diverged merge succeeds");
    let MergeOutcome::Merged { commit_id } = outcome else {
        panic!("a diverged merge creates a merge commit, got {outcome:?}");
    };
    assert_eq!(
        run_git(&repository_path, &["rev-parse", "HEAD"]).trim(),
        commit_id
    );
    let parents = run_git(&repository_path, &["log", "-1", "--format=%P"]);
    assert_eq!(
        parents.split_whitespace().count(),
        2,
        "the merge commit has two parents: {parents:?}"
    );
    assert!(status_porcelain(&repository_path).trim().is_empty());
}

#[test]
fn commit_changed_files_lists_the_branch_side_files_of_a_merge_commit() {
    let repository_path = diverged_repository("merge-commit-files");
    let git = open(&repository_path);
    let outcome = git
        .merge_branch(FEATURE_BRANCH)
        .expect("a diverged merge succeeds");
    let MergeOutcome::Merged { commit_id } = outcome else {
        panic!("a diverged merge creates a merge commit, got {outcome:?}");
    };
    let changed_files = git
        .commit_changed_files(&commit_id)
        .expect("the merge commit files read");
    let paths: Vec<&str> = changed_files
        .iter()
        .map(|changed_file| changed_file.path.as_str())
        .collect();
    assert_eq!(paths, vec![FEATURE_ONLY_PATH]);
}

#[test]
fn merge_branch_reports_conflicts_without_failing() {
    let repository_path = conflicting_repository("conflicts");
    let git = open(&repository_path);
    let outcome = git
        .merge_branch(FEATURE_BRANCH)
        .expect("a conflicted merge is a reported outcome, not a failure");
    assert_eq!(
        outcome,
        MergeOutcome::Conflicts {
            paths: vec![String::from(SHARED_PATH)]
        }
    );
    let status = status_porcelain(&repository_path);
    assert!(
        status.contains(UNMERGED_STATUS_PREFIX),
        "the conflict stays in the index: {status}"
    );
    assert!(git.head_info().expect("the head reads").merging);
}

#[test]
fn merge_branch_reports_an_error_when_a_merge_is_in_progress() {
    let repository_path = conflicting_repository("merge-in-progress");
    let git = open(&repository_path);
    git.merge_branch(FEATURE_BRANCH)
        .expect("the first merge reports its conflicts");
    match git.merge_branch(MAIN_BRANCH) {
        Err(GitError::MergeInProgress) => {}
        other => panic!("a second merge must be refused, got {other:?}"),
    }
    assert!(
        status_porcelain(&repository_path).contains(UNMERGED_STATUS_PREFIX),
        "the refused merge leaves the conflict untouched"
    );
}

#[test]
fn head_info_reports_merging_during_a_conflicted_merge() {
    let repository_path = conflicting_repository("head-info-merging");
    let git = open(&repository_path);
    assert!(!git.head_info().expect("the head reads").merging);
    git.merge_branch(FEATURE_BRANCH)
        .expect("the merge reports its conflicts");
    assert!(git.head_info().expect("the head reads").merging);
}

#[test]
fn committing_without_paths_concludes_a_merge() {
    let repository_path = conflicting_repository("conclude-merge");
    let git = open(&repository_path);
    git.merge_branch(FEATURE_BRANCH)
        .expect("the merge reports its conflicts");
    git.resolve_conflict(SHARED_PATH, ConflictResolution::Ours)
        .expect("accepting our side succeeds");
    let outcome = git
        .commit(CommitRequest {
            message: String::from(MERGE_MESSAGE),
            amend: false,
            sign_off: false,
            paths: Vec::new(),
            hunks: Vec::new(),
        })
        .expect("an empty path list concludes the merge");
    assert_eq!(
        run_git(&repository_path, &["rev-parse", "HEAD"]).trim(),
        outcome.commit_id
    );
    let parents = run_git(&repository_path, &["log", "-1", "--format=%P"]);
    assert_eq!(
        parents.split_whitespace().count(),
        2,
        "the concluding commit has two parents: {parents:?}"
    );
    let status = status_porcelain(&repository_path);
    assert!(
        !status.contains(UNMERGED_STATUS_PREFIX),
        "no unmerged path is left: {status}"
    );
    assert!(!git.head_info().expect("the head reads").merging);
}

#[test]
fn committing_without_paths_still_fails_without_a_merge() {
    let repository_path = diverged_repository("no-merge");
    let git = open(&repository_path);
    let result = git.commit(CommitRequest {
        message: String::from(MERGE_MESSAGE),
        amend: false,
        sign_off: false,
        paths: Vec::new(),
        hunks: Vec::new(),
    });
    assert!(
        matches!(result, Err(GitError::NothingToCommit)),
        "an empty path list still needs a merge to commit, got {result:?}"
    );
}

#[test]
fn a_merge_commit_includes_every_resolved_path() {
    let (repository_path, git) = resolved_two_conflict_merge("merge-index-every-path");
    let outcome = git
        .commit(commit_request_with_paths(MERGE_MESSAGE, &[SHARED_PATH]))
        .expect("the merge commit succeeds even with a partial checkbox set");
    assert_eq!(
        run_git(&repository_path, &["rev-parse", "HEAD"]).trim(),
        outcome.commit_id
    );
    assert_eq!(
        committed_file(&repository_path, SHARED_PATH),
        THEIR_CONTENT,
        "the checked conflict keeps its resolved content"
    );
    assert_eq!(
        committed_file(&repository_path, SECOND_SHARED_PATH),
        SECOND_OUR_CONTENT,
        "the unchecked conflict keeps its resolved content instead of the head content"
    );
    assert_eq!(
        committed_file(&repository_path, CLEAN_PATH),
        CLEAN_THEIR_CONTENT,
        "the unchecked auto-merged path keeps the merge result instead of the head content"
    );
    assert_eq!(parent_count(&repository_path), 2);
    assert!(
        status_porcelain(&repository_path).trim().is_empty(),
        "the concluded merge leaves no staged change behind: {}",
        status_porcelain(&repository_path)
    );
    let changed_paths = run_git(
        &repository_path,
        &["show", "--name-only", "--first-parent", "--format=", "HEAD"],
    );
    let mut changed_paths: Vec<&str> = changed_paths.lines().collect();
    changed_paths.sort_unstable();
    let mut expected_paths = vec![SHARED_PATH, CLEAN_PATH];
    expected_paths.sort_unstable();
    assert_eq!(changed_paths, expected_paths);
}

#[test]
fn a_merge_commit_ignores_the_hunk_selection() {
    let (repository_path, git) = resolved_two_conflict_merge("merge-index-hunk-selection");
    let request = CommitRequest {
        message: String::from(MERGE_MESSAGE),
        amend: false,
        sign_off: false,
        paths: vec![String::from(SECOND_SHARED_PATH), String::from(CLEAN_PATH)],
        hunks: vec![(String::from(CLEAN_PATH), vec![0])],
    };
    git.commit(request)
        .expect("a merge commit ignores the hunk selection");
    assert_eq!(
        committed_file(&repository_path, CLEAN_PATH),
        CLEAN_THEIR_CONTENT,
        "the whole merged file commits instead of the selected hunk"
    );
    assert_eq!(
        committed_file(&repository_path, SHARED_PATH),
        THEIR_CONTENT,
        "a path outside the selection still commits"
    );
    assert_eq!(parent_count(&repository_path), 2);
    assert!(
        status_porcelain(&repository_path).trim().is_empty(),
        "the concluded merge leaves no staged change behind: {}",
        status_porcelain(&repository_path)
    );
}

#[test]
fn a_merge_commit_with_unresolved_paths_reports_an_error() {
    let repository_path = two_conflict_merge_repository("merge-index-unresolved");
    let git = open(&repository_path);
    git.merge_branch(FEATURE_BRANCH)
        .expect("the merge reports its conflicts");
    git.resolve_conflict(SHARED_PATH, ConflictResolution::Theirs)
        .expect("only the first conflict resolves");
    let head_before = run_git(&repository_path, &["rev-parse", "HEAD"]);
    let error = git
        .commit(commit_request_with_paths(MERGE_MESSAGE, &[SHARED_PATH]))
        .expect_err("a merge with an unresolved path cannot commit");
    let message = error.to_string();
    assert!(
        matches!(error, GitError::CommandFailed { .. }),
        "the refusal comes from git: {message}"
    );
    assert!(
        message.contains("unmerged"),
        "the refusal text comes from git: {message}"
    );
    assert_eq!(
        run_git(&repository_path, &["rev-parse", "HEAD"]),
        head_before,
        "the refused commit leaves the head alone"
    );
    assert!(
        status_porcelain(&repository_path).contains(UNMERGED_STATUS_PREFIX),
        "the unresolved path stays conflicted: {}",
        status_porcelain(&repository_path)
    );
    assert!(git.head_info().expect("the head reads").merging);
}

#[test]
fn a_merge_commit_with_amend_keeps_the_second_parent() {
    let (repository_path, git) = resolved_two_conflict_merge("merge-index-amend");
    git.commit(commit_request_with_paths(MERGE_MESSAGE, &[]))
        .expect("the merge commits");
    assert_eq!(parent_count(&repository_path), 2);
    let request = CommitRequest {
        amend: true,
        ..commit_request_with_paths("reworded merge", &[])
    };
    git.commit(request).expect("the amend succeeds");
    assert_eq!(
        run_git(&repository_path, &["log", "-1", "--format=%s"]).trim(),
        "reworded merge"
    );
    assert_eq!(
        parent_count(&repository_path),
        2,
        "the amended merge commit keeps both parents"
    );
    assert_eq!(
        committed_file(&repository_path, CLEAN_PATH),
        CLEAN_THEIR_CONTENT,
        "the amend keeps the merge tree"
    );
    assert!(status_porcelain(&repository_path).trim().is_empty());
}

#[test]
fn a_plain_commit_still_uses_the_temporary_index() {
    let repository_path = scratch_repository("plain-commit-temporary-index");
    write_file(&repository_path, PLAIN_FIRST_PATH, "first staged\n");
    write_file(&repository_path, PLAIN_SECOND_PATH, "second staged\n");
    run_git(&repository_path, &["add", "--all"]);
    let git = open(&repository_path);
    git.commit(commit_request_with_paths(
        "commit only the first",
        &[PLAIN_FIRST_PATH],
    ))
    .expect("the checked path commits");
    let committed = run_git(
        &repository_path,
        &["show", "--name-only", "--format=", "HEAD"],
    );
    assert_eq!(
        committed.trim(),
        PLAIN_FIRST_PATH,
        "the unchecked path stays out of a plain commit"
    );
    assert_eq!(
        run_git(&repository_path, &["diff", "--cached", "--name-only"]).trim(),
        PLAIN_SECOND_PATH,
        "the unchecked path stays staged"
    );
}
