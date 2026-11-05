use gitdeck_core::{LogFilter, LogQuery};
use gitdeck_git::{CliGit, DiffLineKind, FileDiff, GitBackend};
use gitdeck_testkit::{fixture_repo, FixtureRepo};

const MERGE_COMMIT_FILE_PATH: &str = "tracked.txt";
const UNCHANGED_FILE_PATH: &str = "notes.txt";
const CJK_COMMIT_FILE_PATH: &str = "文档/说明.md";
const ROOT_COMMIT_FILE_PATH: &str = "docs/readme.md";

fn repository(fixture: FixtureRepo) -> CliGit {
    CliGit::open(fixture_repo(fixture)).expect("the fixture repository opens")
}

fn commit_ids(repository: &CliGit, count: usize) -> Vec<String> {
    repository
        .log(
            LogQuery {
                max_commits: count,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("the fixture log is readable")
        .into_iter()
        .map(|commit| commit.id)
        .collect()
}

fn added_line_count(diff: &FileDiff) -> usize {
    diff.hunks
        .iter()
        .flat_map(|hunk| hunk.lines.iter())
        .filter(|line| line.kind == DiffLineKind::Added)
        .count()
}

#[test]
fn a_root_commit_diff_lists_every_line_as_added() {
    let repository = repository(FixtureRepo::Dirty);
    let root_commit = commit_ids(&repository, 2)
        .into_iter()
        .nth(1)
        .expect("the fixture has a root commit");
    let diff = repository
        .diff_commit_file(&root_commit, ROOT_COMMIT_FILE_PATH)
        .expect("the root commit diff is readable");
    assert_eq!(diff.path, ROOT_COMMIT_FILE_PATH);
    assert!(!diff.hunks.is_empty(), "the root commit adds the file");
    assert_eq!(added_line_count(&diff), 1);
}

#[test]
fn a_parented_commit_diff_holds_its_own_change() {
    let repository = repository(FixtureRepo::Dirty);
    let head_commit = commit_ids(&repository, 1)
        .into_iter()
        .next()
        .expect("the fixture has a head commit");
    let diff = repository
        .diff_commit_file(&head_commit, "src/app.rs")
        .expect("the head commit diff is readable");
    assert!(!diff.hunks.is_empty());
    assert!(added_line_count(&diff) > 0);
    assert!(
        diff.hunks
            .iter()
            .flat_map(|hunk| hunk.lines.iter())
            .any(|line| line.kind == DiffLineKind::Added && line.text.contains("run_twice")),
        "the diff holds the committed line, got {:?}",
        diff.hunks
    );
}

#[test]
fn a_merge_commit_diffs_against_its_first_parent() {
    let repository = repository(FixtureRepo::LargeTenThousand);
    let merge_commit = commit_ids(&repository, 1)
        .into_iter()
        .next()
        .expect("the fixture has a head commit");
    let diff = repository
        .diff_commit_file(&merge_commit, MERGE_COMMIT_FILE_PATH)
        .expect("the merge commit diff is readable");
    assert_eq!(diff.path, MERGE_COMMIT_FILE_PATH);
    assert!(
        !diff.hunks.is_empty(),
        "the merge commit changes {MERGE_COMMIT_FILE_PATH} against its first parent"
    );
    assert!(added_line_count(&diff) > 0);
}

#[test]
fn a_file_the_commit_leaves_alone_reports_no_hunks() {
    let repository = repository(FixtureRepo::Dirty);
    let head_commit = commit_ids(&repository, 1)
        .into_iter()
        .next()
        .expect("the fixture has a head commit");
    let diff = repository
        .diff_commit_file(&head_commit, UNCHANGED_FILE_PATH)
        .expect("an unchanged path is not an error");
    assert_eq!(diff.path, UNCHANGED_FILE_PATH);
    assert!(diff.hunks.is_empty(), "an unchanged file has no hunks");
    assert!(!diff.binary);
}

#[test]
fn a_cjk_commit_path_reads_like_a_working_tree_diff() {
    let repository = repository(FixtureRepo::CjkPaths);
    let head_commit = commit_ids(&repository, 1)
        .into_iter()
        .next()
        .expect("the fixture has a head commit");
    let commit_diff = repository
        .diff_commit_file(&head_commit, CJK_COMMIT_FILE_PATH)
        .expect("the cjk commit diff is readable");
    let working_tree_diff = repository
        .diff_working_tree(CJK_COMMIT_FILE_PATH)
        .expect("the cjk working tree diff is readable");
    assert_eq!(commit_diff.path, CJK_COMMIT_FILE_PATH);
    assert_eq!(working_tree_diff.path, CJK_COMMIT_FILE_PATH);
    assert!(!commit_diff.hunks.is_empty());
    assert_eq!(commit_diff.binary, working_tree_diff.binary);
    assert_eq!(
        commit_diff.missing_newline_at_end,
        working_tree_diff.missing_newline_at_end
    );
    assert!(
        commit_diff
            .hunks
            .iter()
            .flat_map(|hunk| hunk.lines.iter())
            .any(|line| line.text.contains("说明")),
        "the commit diff keeps the cjk text"
    );
}
