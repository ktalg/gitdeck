use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_core::{CommitSummary, LogFilter, LogQuery};
use gitdeck_git::{CliGit, GitBackend, GitError, LOG_FILTER_SCAN_LIMIT};
use gitdeck_testkit::fixture_git_command;
use gitdeck_testkit::scratch_directory;

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/log-filter";
const CURRENT_BRANCH: &str = "main";
const OTHER_BRANCH: &str = "feature/other";
const UNKNOWN_BRANCH: &str = "no-such-branch";
const DEFAULT_AUTHOR_NAME: &str = "gitdeck-author";
const DEFAULT_AUTHOR_EMAIL: &str = "author@gitdeck.invalid";
const PAGE_SIZE: usize = 10;
const UNBOUNDED_COMMITS: usize = 1_000;
const FILTER_SCAN_TEST_BOUND: usize = 30;
const SCAN_LIMIT_TEST_COMMITS: usize = 60;
const PRODUCT_SCAN_LIMIT: usize = 5_000;

fn scratch_repository(name: &str) -> PathBuf {
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    fs::create_dir_all(&target).expect("the scratch repository directory is created");
    let target = target.canonicalize().expect("the scratch path resolves");
    run_git(&target, &["init", "-q", "-b", CURRENT_BRANCH]);
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

fn commit_with_identity(
    repository_path: &Path,
    subject: &str,
    author_name: &str,
    author_email: &str,
) {
    let output = fixture_git_command(repository_path)
        .args(["commit", "-q", "--allow-empty", "-m", subject])
        .env("GIT_AUTHOR_NAME", author_name)
        .env("GIT_AUTHOR_EMAIL", author_email)
        .env("GIT_COMMITTER_NAME", author_name)
        .env("GIT_COMMITTER_EMAIL", author_email)
        .output()
        .unwrap_or_else(|error| panic!("cannot commit {subject:?}: {error}"));
    assert!(
        output.status.success(),
        "git commit {subject:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn commit_subject(repository_path: &Path, subject: &str) {
    commit_with_identity(
        repository_path,
        subject,
        DEFAULT_AUTHOR_NAME,
        DEFAULT_AUTHOR_EMAIL,
    );
}

fn backend_for(repository_path: &Path) -> CliGit {
    CliGit::open(repository_path).expect("the scratch repository opens")
}

fn unfiltered_query() -> LogQuery {
    LogQuery {
        max_commits: UNBOUNDED_COMMITS,
        skip: 0,
    }
}

fn page_query(skip: usize) -> LogQuery {
    LogQuery {
        max_commits: PAGE_SIZE,
        skip,
    }
}

fn text_filter(text: &str) -> LogFilter {
    LogFilter {
        branch: None,
        text: Some(text.to_string()),
        case_sensitive: false,
    }
}

fn branch_filter(branch: &str) -> LogFilter {
    LogFilter {
        branch: Some(branch.to_string()),
        text: None,
        case_sensitive: false,
    }
}

fn logged_commits(
    repository_path: &Path,
    query: LogQuery,
    filter: &LogFilter,
) -> Vec<CommitSummary> {
    backend_for(repository_path)
        .log(query, filter)
        .expect("log read")
}

fn logged_ids(repository_path: &Path, query: LogQuery, filter: &LogFilter) -> Vec<String> {
    logged_commits(repository_path, query, filter)
        .into_iter()
        .map(|commit| commit.id)
        .collect()
}

#[test]
fn text_filter_matches_subject_author_and_short_id() {
    let repository_path = scratch_repository("text-matches");
    commit_with_identity(
        &repository_path,
        "subject needle subject-match",
        DEFAULT_AUTHOR_NAME,
        DEFAULT_AUTHOR_EMAIL,
    );
    commit_with_identity(
        &repository_path,
        "plain second commit",
        "needle author name",
        DEFAULT_AUTHOR_EMAIL,
    );
    commit_with_identity(
        &repository_path,
        "plain third commit",
        DEFAULT_AUTHOR_NAME,
        "needle-email@example.invalid",
    );

    let by_subject = logged_commits(
        &repository_path,
        unfiltered_query(),
        &text_filter("needle subject-match"),
    );
    assert_eq!(by_subject.len(), 1, "{by_subject:?}");
    assert_eq!(by_subject[0].subject, "subject needle subject-match");

    let by_author_name = logged_commits(
        &repository_path,
        unfiltered_query(),
        &text_filter("needle author name"),
    );
    assert_eq!(by_author_name.len(), 1, "{by_author_name:?}");
    assert_eq!(by_author_name[0].author_name, "needle author name");

    let by_author_email = logged_commits(
        &repository_path,
        unfiltered_query(),
        &text_filter("needle-email@"),
    );
    assert_eq!(by_author_email.len(), 1, "{by_author_email:?}");
    assert_eq!(
        by_author_email[0].author_email,
        "needle-email@example.invalid"
    );

    let all_commits = logged_commits(&repository_path, unfiltered_query(), &LogFilter::default());
    assert_eq!(all_commits.len(), 3, "{all_commits:?}");
    let middle_commit = &all_commits[1];
    let by_short_id = logged_commits(
        &repository_path,
        unfiltered_query(),
        &text_filter(&middle_commit.short_id),
    );
    assert_eq!(by_short_id.len(), 1, "{by_short_id:?}");
    assert_eq!(by_short_id[0].id, middle_commit.id);
}

#[test]
fn text_filter_is_case_insensitive() {
    let repository_path = scratch_repository("case-insensitive");
    commit_subject(&repository_path, "Refactor LOG Parser");
    commit_subject(&repository_path, "unrelated maintenance");

    let lowered = logged_commits(
        &repository_path,
        unfiltered_query(),
        &text_filter("log parser"),
    );
    assert_eq!(lowered.len(), 1, "{lowered:?}");
    assert_eq!(lowered[0].subject, "Refactor LOG Parser");

    let upper_cased = logged_commits(
        &repository_path,
        unfiltered_query(),
        &text_filter("REFACTOR"),
    );
    assert_eq!(upper_cased, lowered);
}

#[test]
fn text_filter_with_no_match_returns_empty_without_error() {
    let repository_path = scratch_repository("no-match");
    commit_subject(&repository_path, "first commit");
    commit_subject(&repository_path, "second commit");
    commit_subject(&repository_path, "third commit");

    let matches = backend_for(&repository_path)
        .log(unfiltered_query(), &text_filter("no-such-needle-anywhere"))
        .expect("a filter without matches is not an error");
    assert!(matches.is_empty(), "{matches:?}");
}

#[test]
fn branch_filter_returns_only_that_branch() {
    let repository_path = scratch_repository("branch-filter");
    commit_subject(&repository_path, "shared base commit");
    run_git(&repository_path, &["checkout", "-q", "-b", OTHER_BRANCH]);
    commit_subject(&repository_path, "other branch commit");
    run_git(&repository_path, &["checkout", "-q", CURRENT_BRANCH]);
    commit_subject(&repository_path, "main branch commit");

    let on_main = logged_commits(
        &repository_path,
        unfiltered_query(),
        &branch_filter(CURRENT_BRANCH),
    );
    let on_other = logged_commits(
        &repository_path,
        unfiltered_query(),
        &branch_filter(OTHER_BRANCH),
    );

    let main_subjects: Vec<&str> = on_main
        .iter()
        .map(|commit| commit.subject.as_str())
        .collect();
    let other_subjects: Vec<&str> = on_other
        .iter()
        .map(|commit| commit.subject.as_str())
        .collect();
    assert_eq!(main_subjects, ["main branch commit", "shared base commit"]);
    assert_eq!(
        other_subjects,
        ["other branch commit", "shared base commit"]
    );
}

#[test]
fn branch_filter_with_unknown_name_reports_an_error() {
    let repository_path = scratch_repository("unknown-branch");
    commit_subject(&repository_path, "only commit");

    let error = backend_for(&repository_path)
        .log(unfiltered_query(), &branch_filter(UNKNOWN_BRANCH))
        .expect_err("an unknown branch is not silently empty");
    match error {
        GitError::CommandFailed {
            arguments, message, ..
        } => {
            assert!(
                arguments.starts_with("log "),
                "the failure names the log command, got {arguments:?}"
            );
            assert!(
                message.contains(UNKNOWN_BRANCH),
                "the failure names the unknown branch, got {message:?}"
            );
        }
        other => panic!("expected CommandFailed, got {other:?}"),
    }
}

#[test]
fn paging_after_filter_has_no_gaps_or_duplicates() {
    let repository_path = scratch_repository("paging-after-filter");
    let needle_commit_count = 25;
    for index in 0..needle_commit_count {
        commit_subject(&repository_path, &format!("needle commit {index:02}"));
        if index % 3 == 0 {
            commit_subject(&repository_path, &format!("plain commit {index:02}"));
        }
    }

    let filter = text_filter("needle commit");
    let all_matching_ids = logged_ids(&repository_path, unfiltered_query(), &filter);
    assert_eq!(
        all_matching_ids.len(),
        needle_commit_count,
        "{all_matching_ids:?}"
    );

    let mut paged_ids = Vec::new();
    let mut page_lengths = Vec::new();
    for skip in [0, PAGE_SIZE, 2 * PAGE_SIZE] {
        let page = logged_ids(&repository_path, page_query(skip), &filter);
        page_lengths.push(page.len());
        paged_ids.extend(page);
    }
    assert_eq!(page_lengths, [PAGE_SIZE, PAGE_SIZE, 5]);
    if paged_ids != all_matching_ids {
        let first_mismatch = paged_ids
            .iter()
            .zip(&all_matching_ids)
            .position(|(paged, expected)| paged != expected);
        panic!(
            "paging diverges from the filtered log at index {first_mismatch:?}, paged {} commits, expected {}",
            paged_ids.len(),
            all_matching_ids.len()
        );
    }
    let distinct_ids: std::collections::HashSet<&String> = paged_ids.iter().collect();
    assert_eq!(distinct_ids.len(), needle_commit_count);
}

#[test]
fn empty_filter_equals_unfiltered_log() {
    let repository_path = scratch_repository("empty-filter");
    for index in 0..5 {
        commit_subject(&repository_path, &format!("commit {index}"));
    }
    let committed_count = run_git(&repository_path, &["rev-list", "--count", "HEAD"])
        .trim()
        .parse::<usize>()
        .expect("rev-list reports the commit count");

    let default_filtered = logged_ids(&repository_path, unfiltered_query(), &LogFilter::default());
    let empty_text_filtered = logged_ids(
        &repository_path,
        unfiltered_query(),
        &LogFilter {
            branch: None,
            text: Some(String::new()),
            case_sensitive: false,
        },
    );
    assert_eq!(default_filtered, empty_text_filtered);
    assert_eq!(default_filtered.len(), committed_count);
}

#[test]
fn scan_limit_stops_after_the_bound() {
    let repository_path = scratch_repository("scan-limit");
    commit_subject(&repository_path, "needle at the bottom");
    for index in 0..SCAN_LIMIT_TEST_COMMITS - 1 {
        commit_subject(&repository_path, &format!("plain commit {index:02}"));
    }
    assert_eq!(LOG_FILTER_SCAN_LIMIT, PRODUCT_SCAN_LIMIT);

    let filter = text_filter("needle at the bottom");
    let backend = backend_for(&repository_path);
    let bounded_to_the_newest_commits = backend
        .log_with_scan_limit(unfiltered_query(), &filter, FILTER_SCAN_TEST_BOUND)
        .expect("bounded log read");
    assert!(
        bounded_to_the_newest_commits.is_empty(),
        "the filter scanned past its bound: {bounded_to_the_newest_commits:?}"
    );

    let bounded_to_the_whole_history = backend
        .log_with_scan_limit(unfiltered_query(), &filter, SCAN_LIMIT_TEST_COMMITS)
        .expect("bounded log read");
    assert_eq!(bounded_to_the_whole_history.len(), 1);
    assert_eq!(
        bounded_to_the_whole_history[0].subject,
        "needle at the bottom"
    );

    let default_limited = backend.log(unfiltered_query(), &filter).expect("log read");
    assert_eq!(default_limited, bounded_to_the_whole_history);
}
