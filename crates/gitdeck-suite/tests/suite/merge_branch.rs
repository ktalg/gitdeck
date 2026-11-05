use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::App;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{PanelErrorSource, StatusTone, MERGE_FAILED_LABEL, MERGING_LABEL};

const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const CURRENT_BRANCH: &str = "main";
const LOCAL_BRANCH: &str = "feature/local";
const CURRENT_ONLY_PATH: &str = "src/main.rs";
const BRANCH_ONLY_PATH: &str = "src/feature.rs";
const SHARED_PATH: &str = "src/shared.rs";
const BASE_CONTENT: &str = "fn shared() {\n    base();\n}\n";
const CURRENT_CONTENT: &str = "fn shared() {\n    current();\n}\n";
const BRANCH_CONTENT: &str = "fn shared() {\n    branch();\n}\n";
const MERGED_RECEIPT: &str = "Merged feature/local into main";
const CONFLICT_RECEIPT: &str = "Merge of feature/local stopped with conflicts";

fn run_git(repository_path: &Path, arguments: &[&str]) {
    let output = fixture_git_command(repository_path)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn commit_file(repository_path: &Path, relative_path: &str, content: &str, message: &str) {
    let path = repository_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the parent directory is created");
    }
    fs::write(&path, content).expect("the scratch file is written");
    run_git(repository_path, &["add", "--all"]);
    run_git(repository_path, &["commit", "-q", "-m", message]);
}

fn scratch_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, &["init", "-q", "-b", CURRENT_BRANCH]);
    commit_file(&path, SHARED_PATH, BASE_CONTENT, "base commit");
    path
}

fn diverged_repository(name: &str) -> PathBuf {
    let path = scratch_repository(name);
    run_git(&path, &["switch", "-q", "-c", LOCAL_BRANCH]);
    commit_file(
        &path,
        BRANCH_ONLY_PATH,
        "fn feature() {}\n",
        "branch commit",
    );
    run_git(&path, &["switch", "-q", CURRENT_BRANCH]);
    commit_file(&path, CURRENT_ONLY_PATH, "fn main() {}\n", "current commit");
    path
}

fn conflicting_repository(name: &str) -> PathBuf {
    let path = scratch_repository(name);
    run_git(&path, &["switch", "-q", "-c", LOCAL_BRANCH]);
    commit_file(&path, SHARED_PATH, BRANCH_CONTENT, "branch commit");
    run_git(&path, &["switch", "-q", CURRENT_BRANCH]);
    commit_file(&path, SHARED_PATH, CURRENT_CONTENT, "current commit");
    path
}

fn merge_terminal(repository_path: &Path) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(
        App::with_repository(repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        MEDIUM_WIDTH,
        MEDIUM_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(MEDIUM_WIDTH, MEDIUM_HEIGHT);
    terminal
}

fn select_the_branch(terminal: &mut TestTerminal<App>, branch: &str) {
    terminal.send_keys("b");
    terminal.tick();
    let branches = terminal.driver().branches().to_vec();
    let index = branches
        .iter()
        .position(|candidate| candidate.name == branch)
        .unwrap_or_else(|| panic!("the branch list holds {branch}: {branches:?}"));
    let current = terminal
        .driver()
        .selected_branch_index()
        .expect("a branch is selected");
    for _ in current..index {
        terminal.send_keys("down");
        terminal.tick();
    }
    for _ in index..current {
        terminal.send_keys("up");
        terminal.tick();
    }
    assert_eq!(terminal.driver().selected_branch_index(), Some(index));
}

fn head_parent_count(repository_path: &Path) -> usize {
    let output = fixture_git_command(repository_path)
        .args(["log", "-1", "--format=%P"])
        .output()
        .expect("git log runs");
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .count()
}

#[test]
fn alt_m_merges_the_selected_branch_and_reports_it() {
    for caps in [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic] {
        let repository_path = diverged_repository("alt-m-merge");
        let mut terminal = TestTerminal::new(
            App::with_repository(&repository_path, caps, ThemeId::JbDark)
                .expect("the scratch repository opens"),
            MEDIUM_WIDTH,
            MEDIUM_HEIGHT,
            caps,
            ThemeId::JbDark,
        );
        terminal.resize(MEDIUM_WIDTH, MEDIUM_HEIGHT);
        select_the_branch(&mut terminal, LOCAL_BRANCH);
        terminal.send_keys("alt+m");
        terminal.tick();
        assert_eq!(
            terminal.driver().status_message(),
            Some((MERGED_RECEIPT, StatusTone::Info)),
            "{caps:?} the status bar reports the merge"
        );
        assert_eq!(
            head_parent_count(&repository_path),
            2,
            "{caps:?} the merge commit has two parents"
        );
        terminal.screenshot(&format!("merge_branch/alt-m-merged-{caps:?}"));
    }
}

#[test]
fn alt_m_on_a_conflicting_branch_shows_the_merge_failed_block() {
    let repository_path = conflicting_repository("alt-m-conflict");
    let mut terminal = merge_terminal(&repository_path);
    select_the_branch(&mut terminal, LOCAL_BRANCH);
    terminal.send_keys("alt+m");
    terminal.tick();
    terminal.tick();
    assert_eq!(
        terminal.driver().panel_error(),
        Some((PanelErrorSource::Merge, "1 file has conflicts")),
        "the conflicted merge is titled Merge failed"
    );
    assert_eq!(
        terminal.driver().status_message(),
        Some((CONFLICT_RECEIPT, StatusTone::Failure))
    );
    let screen_text = terminal.screen_text();
    assert!(
        screen_text.contains(MERGE_FAILED_LABEL),
        "the error block is titled Merge failed, screen was:\n{screen_text}"
    );
    assert!(
        screen_text.contains(MERGING_LABEL),
        "the footer reports the merge in progress, screen was:\n{screen_text}"
    );
    assert!(
        screen_text.contains("Merge Conflicts"),
        "the conflicted file lands in the merge conflicts group, screen was:\n{screen_text}"
    );
    terminal.screenshot("merge_branch/alt-m-conflicts");
}

#[test]
fn alt_m_on_a_branch_already_merged_reports_up_to_date() {
    let repository_path = diverged_repository("alt-m-up-to-date");
    let mut terminal = merge_terminal(&repository_path);
    select_the_branch(&mut terminal, LOCAL_BRANCH);
    terminal.send_keys("alt+m");
    terminal.tick();
    terminal.send_keys("alt+m");
    terminal.tick();
    assert_eq!(
        terminal.driver().status_message(),
        Some(("Already up to date", StatusTone::Info)),
        "a second merge of the same branch reports up to date"
    );
    assert!(
        !terminal.screen_text().contains(MERGING_LABEL),
        "a finished merge leaves no merging marker, screen was:\n{}",
        terminal.screen_text()
    );
}

#[test]
fn the_merge_message_is_prefilled_from_merge_msg() {
    let repository_path = conflicting_repository("merge-msg-prefill");
    let mut terminal = merge_terminal(&repository_path);
    select_the_branch(&mut terminal, LOCAL_BRANCH);
    terminal.send_keys("alt+m");
    terminal.tick();
    terminal.tick();
    assert_eq!(
        terminal.driver().commit_message(),
        "Merge branch 'feature/local'",
        "the merge message box carries the git generated merge message, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("merge_branch/merge-message-prefill");
}

#[test]
fn a_merge_commit_reports_a_merge_receipt() {
    let repository_path = conflicting_repository("merge-commit-receipt");
    let mut terminal = merge_terminal(&repository_path);
    select_the_branch(&mut terminal, LOCAL_BRANCH);
    terminal.send_keys("alt+m");
    terminal.tick();
    terminal.tick();
    run_git(&repository_path, &["add", SHARED_PATH]);
    terminal.send_keys("f5");
    terminal.tick();
    terminal.send_keys("ctrl+k");
    terminal.send_keys("ctrl+enter");
    terminal.tick();
    terminal.tick();
    assert_eq!(
        terminal.driver().status_message(),
        Some(("main merged and committed", StatusTone::Info)),
        "the merge commit reports the merge instead of the file count, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(
        terminal
            .driver()
            .commits()
            .first()
            .is_some_and(|commit| commit.subject.starts_with("Merge branch 'feature/local'")),
        "git records the merge commit with the prefilled message"
    );
    terminal.screenshot("merge_branch/merge-commit-receipt");
}
