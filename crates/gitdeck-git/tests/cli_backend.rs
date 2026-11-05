use std::collections::HashSet;
use std::time::{Duration, Instant};

use gitdeck_core::{BranchEntry, ChangeEntry, ChangeKind, LogFilter, LogQuery, RefKind};
use gitdeck_git::{ChangedFile, CliGit, DiffLineKind, FileDiff, GitBackend, GitError};
use gitdeck_testkit::{fixture_repo, FixtureRepo};

const LOG_PAGE_SIZE: usize = 200;
const LOG_PAGE_TIME_LIMIT: Duration = Duration::from_millis(500);
const SHORT_ID_LENGTH: usize = 7;
const LARGE_DIFF_TIME_LIMIT: Duration = Duration::from_millis(300);
const LARGE_DIFF_HUNK_COUNT: usize = 10;

fn open_fixture(fixture: FixtureRepo) -> CliGit {
    CliGit::open(fixture_repo(fixture)).expect("fixture repository opens")
}

#[test]
fn dirty_fixture_reports_modified_and_untracked_changes() {
    let git = open_fixture(FixtureRepo::Dirty);
    let changes = git.changes().expect("changes read");
    assert!(
        changes
            .iter()
            .any(|change| change.kind == ChangeKind::Modified),
        "{changes:?}"
    );
    assert!(
        changes
            .iter()
            .any(|change| change.kind == ChangeKind::Untracked),
        "{changes:?}"
    );
    let untracked = changes
        .iter()
        .find(|change| change.kind == ChangeKind::Untracked)
        .expect("untracked change");
    assert_eq!(untracked.path, "notes.txt");
    assert!(!untracked.staged);
    let staged_addition = changes
        .iter()
        .find(|change| change.path == "src/module.rs")
        .expect("staged addition");
    assert_eq!(staged_addition.kind, ChangeKind::Added);
    assert!(staged_addition.staged);
    let head = git.head_info().expect("head info read");
    assert_eq!(head.branch.as_deref(), Some("main"));
    assert!(!head.detached);
    assert_eq!(head.upstream, None);
    assert_eq!((head.ahead, head.behind), (0, 0));
}

#[test]
fn dirty_fixture_log_holds_head_and_branch_references() {
    let git = open_fixture(FixtureRepo::Dirty);
    let commits = git
        .log(
            LogQuery {
                max_commits: 10,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("log read");
    assert_eq!(commits.len(), 2, "{commits:?}");
    let head = &commits[0];
    assert_eq!(head.id.len(), 40);
    assert_eq!(head.short_id.len(), SHORT_ID_LENGTH);
    assert_eq!(head.subject, "call run twice");
    assert_eq!(head.author_name, "gitdeck-fixture");
    assert_eq!(head.author_email, "fixture@gitdeck.invalid");
    assert_eq!(head.parent_ids.len(), 1);
    assert!(commits[1].parent_ids.is_empty());
    let head_reference = head
        .refs
        .iter()
        .find(|name| name.kind == RefKind::Head)
        .expect("HEAD reference");
    assert_eq!(head_reference.full, "HEAD");
    assert!(head.refs.iter().any(|name| {
        name.kind == RefKind::LocalBranch && name.full == "refs/heads/main" && name.short == "main"
    }));
}

#[test]
fn dirty_fixture_branches_mark_the_current_branch() {
    let git = open_fixture(FixtureRepo::Dirty);
    let branches = git.branches().expect("branches read");
    assert_eq!(
        branches,
        vec![BranchEntry {
            name: String::from("main"),
            kind: RefKind::LocalBranch,
            is_head: true,
            upstream: None,
        }]
    );
}

#[test]
fn merge_conflict_fixture_reports_a_conflicted_change() {
    let git = open_fixture(FixtureRepo::MergeConflict);
    let changes = git.changes().expect("changes read");
    let conflicted: Vec<&ChangeEntry> = changes
        .iter()
        .filter(|change| change.kind == ChangeKind::Conflicted)
        .collect();
    assert_eq!(conflicted.len(), 1, "{changes:?}");
    assert_eq!(conflicted[0].path, "src/conflict.txt");
    assert!(!conflicted[0].staged);
}

#[test]
fn cjk_paths_are_decoded_verbatim() {
    let git = open_fixture(FixtureRepo::CjkPaths);
    let changes = git.changes().expect("changes read");
    assert_eq!(changes.len(), 2, "{changes:?}");
    let untracked = changes
        .iter()
        .find(|change| change.path == "新增/未跟踪 文件.txt")
        .expect("untracked wide character path");
    assert_eq!(untracked.kind, ChangeKind::Untracked);
    assert!(!untracked.staged);
    assert_eq!(untracked.previous_path, None);
    let modified = changes
        .iter()
        .find(|change| change.path == "文档/说明.md")
        .expect("modified wide character path");
    assert_eq!(modified.kind, ChangeKind::Modified);
    for change in &changes {
        assert!(
            !change.path.contains("\\3"),
            "{:?} holds an octal escaped path",
            change.path
        );
    }
    let branches = git.branches().expect("branches read");
    assert!(branches.iter().any(|branch| branch.name == "main"));
    let commits = git
        .log(
            LogQuery {
                max_commits: 10,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("log read");
    assert_eq!(commits.len(), 2, "{commits:?}");
    assert_eq!(commits[0].subject, "更新：说明文档");
    assert_eq!(commits[1].subject, "添加中文路径与中文提交信息");
}

#[test]
fn large_fixture_log_pages_are_newest_first_and_cheap() {
    let git = open_fixture(FixtureRepo::LargeTenThousand);
    let started_at = Instant::now();
    let first_page = git
        .log(
            LogQuery {
                max_commits: LOG_PAGE_SIZE,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("first log page");
    let elapsed = started_at.elapsed();
    println!("large-10k first log page of {LOG_PAGE_SIZE} commits took {elapsed:?}");
    assert_eq!(first_page.len(), LOG_PAGE_SIZE);
    assert!(
        elapsed < LOG_PAGE_TIME_LIMIT,
        "first log page took {elapsed:?}, limit is {LOG_PAGE_TIME_LIMIT:?}"
    );
    assert!(
        first_page
            .windows(2)
            .all(|pair| pair[0].author_timestamp >= pair[1].author_timestamp),
        "first log page is not newest first"
    );
    let second_page = git
        .log(
            LogQuery {
                max_commits: LOG_PAGE_SIZE,
                skip: LOG_PAGE_SIZE,
            },
            &LogFilter::default(),
        )
        .expect("second log page");
    assert_eq!(second_page.len(), LOG_PAGE_SIZE);
    let first_page_ids: HashSet<&str> =
        first_page.iter().map(|commit| commit.id.as_str()).collect();
    assert_eq!(first_page_ids.len(), LOG_PAGE_SIZE);
    assert!(
        second_page
            .iter()
            .all(|commit| !first_page_ids.contains(commit.id.as_str())),
        "second log page overlaps the first"
    );
}

#[test]
fn large_fixture_filtered_log_matches_text_within_the_time_limit() {
    let git = open_fixture(FixtureRepo::LargeTenThousand);
    let filter = LogFilter {
        branch: None,
        text: Some(String::from("commit 99")),
        case_sensitive: false,
    };
    let started_at = Instant::now();
    let commits = git
        .log(
            LogQuery {
                max_commits: 10,
                skip: 0,
            },
            &filter,
        )
        .expect("filtered log read");
    let elapsed = started_at.elapsed();
    println!("large-10k filtered log of 10 commits took {elapsed:?}");
    assert_eq!(commits.len(), 10, "{commits:?}");
    assert!(commits
        .iter()
        .all(|commit| commit.subject.contains("commit 99")));
    assert!(
        elapsed < LOG_PAGE_TIME_LIMIT,
        "filtered log took {elapsed:?}, limit is {LOG_PAGE_TIME_LIMIT:?}"
    );
}

#[test]
fn detached_head_fixture_reports_a_detached_head_without_branch() {
    let git = open_fixture(FixtureRepo::DetachedHead);
    let head = git.head_info().expect("head info read");
    assert!(head.detached);
    assert_eq!(head.branch, None);
    assert_eq!(head.upstream, None);
    let branches = git.branches().expect("branches read");
    assert!(
        branches.iter().all(|branch| !branch.is_head),
        "{branches:?}"
    );
    assert!(branches.iter().any(|branch| branch.name == "main"));
    let commits = git
        .log(
            LogQuery {
                max_commits: 10,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("log read");
    assert_eq!(commits.len(), 2);
    assert!(commits[0]
        .refs
        .iter()
        .any(|name| name.kind == RefKind::Head));
}

#[test]
fn empty_fixture_opens_without_changes_or_commits() {
    let git = open_fixture(FixtureRepo::Empty);
    assert!(git.changes().expect("changes read").is_empty());
    assert!(git
        .log(
            LogQuery {
                max_commits: 50,
                skip: 0,
            },
            &LogFilter::default()
        )
        .expect("log read")
        .is_empty());
    assert!(git.branches().expect("branches read").is_empty());
    let head = git.head_info().expect("head info read");
    assert_eq!(head.branch.as_deref(), Some("main"));
    assert!(!head.detached);
    assert_eq!((head.ahead, head.behind), (0, 0));
}

#[test]
fn opening_a_missing_directory_reports_the_path_and_the_git_error_line() {
    let error = CliGit::open("/nonexistent").expect_err("missing directory must not open");
    assert!(matches!(error, GitError::CommandFailed { .. }), "{error:?}");
    let message = error.to_string();
    assert!(message.contains("/nonexistent"), "{message}");
    assert!(message.contains("rev-parse"), "{message}");
}

fn counted_lines(file_diff: &FileDiff, line_kind: DiffLineKind) -> usize {
    file_diff
        .hunks
        .iter()
        .flat_map(|hunk| hunk.lines.iter())
        .filter(|line| line.kind == line_kind)
        .count()
}

#[test]
fn diff_showcase_modified_file_reports_three_changes_a_deletion_and_an_addition() {
    let git = open_fixture(FixtureRepo::DiffShowcase);
    let file_diff = git
        .diff_working_tree("src/modified.rs")
        .expect("the modified diff reads");
    assert_eq!(file_diff.path, "src/modified.rs");
    assert!(!file_diff.binary);
    assert!(!file_diff.missing_newline_at_end);
    assert_eq!(counted_lines(&file_diff, DiffLineKind::Added), 4);
    assert_eq!(counted_lines(&file_diff, DiffLineKind::Removed), 4);
    assert!(counted_lines(&file_diff, DiffLineKind::Context) > 0);
    assert!(file_diff
        .hunks
        .iter()
        .all(|hunk| hunk.lines.iter().all(|line| !line.text.contains('\r'))));
}

#[test]
fn diff_showcase_untracked_file_reports_only_added_lines() {
    let git = open_fixture(FixtureRepo::DiffShowcase);
    let file_diff = git
        .diff_working_tree("src/untracked_new.rs")
        .expect("the untracked diff reads");
    assert_eq!(file_diff.hunks.len(), 1);
    assert_eq!(counted_lines(&file_diff, DiffLineKind::Added), 10);
    assert_eq!(counted_lines(&file_diff, DiffLineKind::Removed), 0);
    assert_eq!(counted_lines(&file_diff, DiffLineKind::Context), 0);
    assert!(file_diff.hunks[0]
        .lines
        .iter()
        .all(|line| line.old_line_number.is_none() && line.new_line_number.is_some()));
}

#[test]
fn diff_showcase_binary_file_reports_the_binary_flag_without_hunks() {
    let git = open_fixture(FixtureRepo::DiffShowcase);
    let file_diff = git
        .diff_working_tree("assets/logo.bin")
        .expect("the binary diff reads");
    assert!(file_diff.binary);
    assert!(file_diff.hunks.is_empty());
}

#[test]
fn diff_showcase_missing_trailing_newline_is_reported() {
    let git = open_fixture(FixtureRepo::DiffShowcase);
    let file_diff = git
        .diff_working_tree("src/no_newline.txt")
        .expect("the no newline diff reads");
    assert!(file_diff.missing_newline_at_end);
    let marker_line = file_diff
        .hunks
        .iter()
        .flat_map(|hunk| hunk.lines.iter())
        .find(|line| line.old_line_number.is_none() && line.new_line_number.is_none())
        .expect("the missing newline marker line");
    assert_eq!(marker_line.text, "\\ No newline at end of file");
}

#[test]
fn diff_showcase_wide_character_file_keeps_its_text() {
    let git = open_fixture(FixtureRepo::DiffShowcase);
    let file_diff = git
        .diff_working_tree("文档/说明.md")
        .expect("the wide character diff reads");
    assert_eq!(file_diff.path, "文档/说明.md");
    let texts: Vec<&str> = file_diff
        .hunks
        .iter()
        .flat_map(|hunk| hunk.lines.iter())
        .map(|line| line.text.as_str())
        .collect();
    assert!(texts.contains(&"第一行内容"), "{texts:?}");
    assert!(texts.contains(&"第二行内容"), "{texts:?}");
}

#[test]
fn diff_showcase_deleted_file_reports_only_removed_lines() {
    let git = open_fixture(FixtureRepo::Dirty);
    let file_diff = git
        .diff_working_tree("docs/readme.md")
        .expect("the deleted diff reads");
    assert_eq!(counted_lines(&file_diff, DiffLineKind::Added), 0);
    assert!(counted_lines(&file_diff, DiffLineKind::Removed) > 0);
}

#[test]
fn diff_showcase_large_file_opens_within_the_time_budget() {
    let git = open_fixture(FixtureRepo::DiffShowcase);
    let started_at = Instant::now();
    let file_diff = git
        .diff_working_tree("src/large.txt")
        .expect("the large diff reads");
    let elapsed = started_at.elapsed();
    println!("src/large.txt diff opened in {elapsed:?}");
    assert_eq!(file_diff.hunks.len(), LARGE_DIFF_HUNK_COUNT);
    assert!(
        elapsed < LARGE_DIFF_TIME_LIMIT,
        "opening the large diff took {elapsed:?}, limit is {LARGE_DIFF_TIME_LIMIT:?}"
    );
}

#[test]
fn head_commit_changed_files_list_the_paths_of_the_last_commit() {
    let git = open_fixture(FixtureRepo::Dirty);
    let head_id = git
        .log(
            LogQuery {
                max_commits: 1,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("log read")[0]
        .id
        .clone();
    let changed_files = git
        .commit_changed_files(&head_id)
        .expect("commit changed files read");
    assert_eq!(
        changed_files,
        vec![ChangedFile {
            path: String::from("src/app.rs"),
            previous_path: None,
            kind: ChangeKind::Modified,
        }]
    );
}

#[test]
fn root_commit_changed_files_list_both_initial_additions() {
    let git = open_fixture(FixtureRepo::Dirty);
    let commits = git
        .log(
            LogQuery {
                max_commits: 10,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("log read");
    let root_id = commits
        .last()
        .expect("the dirty fixture holds a root commit")
        .id
        .clone();
    let changed_files = git
        .commit_changed_files(&root_id)
        .expect("commit changed files read");
    assert_eq!(
        changed_files,
        vec![
            ChangedFile {
                path: String::from("docs/readme.md"),
                previous_path: None,
                kind: ChangeKind::Added,
            },
            ChangedFile {
                path: String::from("src/app.rs"),
                previous_path: None,
                kind: ChangeKind::Added,
            },
        ]
    );
}

#[test]
fn merge_commit_reports_its_combined_change_as_modified() {
    let git = open_fixture(FixtureRepo::LargeTenThousand);
    let merge_id = git
        .log(
            LogQuery {
                max_commits: 100,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("log read")
        .into_iter()
        .find(|commit| commit.parent_ids.len() > 1)
        .expect("the large fixture holds a merge commit")
        .id;
    let changed_files = git
        .commit_changed_files(&merge_id)
        .expect("merge commit changed files read");
    assert_eq!(
        changed_files,
        vec![ChangedFile {
            path: String::from("tracked.txt"),
            previous_path: None,
            kind: ChangeKind::Modified,
        }]
    );
}

#[test]
fn unknown_commit_id_reports_the_failing_git_command() {
    let git = open_fixture(FixtureRepo::Dirty);
    let error = git
        .commit_changed_files("0000000000000000000000000000000000000000")
        .expect_err("an unknown commit id must not resolve");
    assert!(matches!(error, GitError::CommandFailed { .. }), "{error:?}");
}
