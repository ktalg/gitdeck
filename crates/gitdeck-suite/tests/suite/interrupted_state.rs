use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::App;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{PanelErrorSource, StatusTone, MERGING_LABEL, REBASING_LABEL};

const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const CURRENT_BRANCH: &str = "main";
const SOURCE_BRANCH: &str = "feature/local";
const SHARED_PATH: &str = "src/interrupted.rs";
const BASE_CONTENT: &str = "fn shared() {\n    base();\n}\n";
const CURRENT_CONTENT: &str = "fn shared() {\n    current();\n}\n";
const BRANCH_CONTENT: &str = "fn shared() {\n    branch();\n}\n";
const CLEAN_PATH: &str = "src/clean.rs";
const CLEAN_BRANCH_CONTENT: &str = "fn clean() {\n    branch();\n}\n";
const MERGE_ABORTED_RECEIPT: &str = "Aborted the merge";
const REBASE_ABORTED_RECEIPT: &str = "Aborted the rebase";
const REBASE_CONTINUE_HINT: &str = "Resolve the conflicts and continue the rebase";

fn git_output(repository_path: &Path, arguments: &[&str]) -> String {
    let output = fixture_git_command(repository_path)
        .args(arguments)
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn run_git(repository_path: &Path, arguments: &[&str]) {
    let status = fixture_git_command(repository_path)
        .args(arguments)
        .env("GIT_EDITOR", "true")
        .status()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        status.success(),
        "git {arguments:?} was expected to succeed"
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

fn diverged_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, &["init", "-q", "-b", CURRENT_BRANCH]);
    commit_file(&path, SHARED_PATH, BASE_CONTENT, "base commit");
    run_git(&path, &["switch", "-q", "-c", SOURCE_BRANCH]);
    commit_file(&path, SHARED_PATH, BRANCH_CONTENT, "branch commit");
    commit_file(
        &path,
        CLEAN_PATH,
        CLEAN_BRANCH_CONTENT,
        "branch clean commit",
    );
    run_git(&path, &["switch", "-q", CURRENT_BRANCH]);
    commit_file(&path, SHARED_PATH, CURRENT_CONTENT, "current commit");
    path
}

fn repository_stopped_in_a_merge(name: &str) -> PathBuf {
    let path = diverged_repository(name);
    git_output(&path, &["merge", SOURCE_BRANCH]);
    assert!(
        path.join(".git").join("MERGE_HEAD").exists(),
        "the merge stops with conflicts"
    );
    path
}

fn repository_stopped_in_a_rebase(name: &str) -> PathBuf {
    let path = diverged_repository(name);
    run_git(&path, &["switch", "-q", SOURCE_BRANCH]);
    git_output(&path, &["rebase", CURRENT_BRANCH]);
    assert!(
        path.join(".git").join("rebase-merge").exists(),
        "the rebase stops with conflicts"
    );
    path
}

fn interrupted_terminal(repository_path: &Path) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(
        App::with_repository(repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        MEDIUM_WIDTH,
        MEDIUM_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(MEDIUM_WIDTH, MEDIUM_HEIGHT);
    terminal.tick();
    terminal
}

#[test]
fn the_footer_shows_the_rebase_state() {
    let rebase_path = repository_stopped_in_a_rebase("footer-rebase");
    let rebase_terminal = interrupted_terminal(&rebase_path);
    let rebase_screen = rebase_terminal.screen_text();
    assert!(
        rebase_screen.contains(REBASING_LABEL),
        "the footer reports the rebase, screen was:\n{rebase_screen}"
    );
    assert!(
        !rebase_screen.contains(MERGING_LABEL),
        "a rebase is not reported as a merge, screen was:\n{rebase_screen}"
    );
    rebase_terminal.screenshot("interrupted_state/footer-rebasing");

    let merge_path = repository_stopped_in_a_merge("footer-merge");
    let merge_terminal = interrupted_terminal(&merge_path);
    let merge_screen = merge_terminal.screen_text();
    assert!(
        merge_screen.contains(MERGING_LABEL),
        "the footer reports the merge, screen was:\n{merge_screen}"
    );
    assert!(
        !merge_screen.contains(REBASING_LABEL),
        "a merge is not reported as a rebase, screen was:\n{merge_screen}"
    );
    merge_terminal.screenshot("interrupted_state/footer-merging");
}

#[test]
fn committing_during_a_rebase_is_refused_with_a_hint() {
    let repository_path = repository_stopped_in_a_rebase("commit-during-rebase");
    let head_before = git_output(&repository_path, &["rev-parse", "HEAD"]);
    let mut terminal = interrupted_terminal(&repository_path);
    terminal.send_keys("ctrl+enter");
    terminal.tick();
    assert_eq!(
        terminal.driver().status_message(),
        Some((REBASE_CONTINUE_HINT, StatusTone::Failure)),
        "the commit is refused with the continue hint, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        git_output(&repository_path, &["rev-parse", "HEAD"]),
        head_before,
        "the refused commit leaves the head alone"
    );
    terminal.screenshot("interrupted_state/commit-during-rebase-refused");
}

#[test]
fn alt_x_aborts_and_reports_it() {
    let merge_path = repository_stopped_in_a_merge("alt-x-merge");
    let mut merge_terminal = interrupted_terminal(&merge_path);
    merge_terminal.send_keys("alt+x");
    merge_terminal.send_keys("tab,enter");
    merge_terminal.tick();
    merge_terminal.tick();
    assert_eq!(
        merge_terminal.driver().status_message(),
        Some((MERGE_ABORTED_RECEIPT, StatusTone::Info)),
        "the status bar reports the aborted merge"
    );
    assert!(
        !merge_terminal.screen_text().contains(MERGING_LABEL),
        "the merging marker disappears, screen was:\n{}",
        merge_terminal.screen_text()
    );
    assert!(
        git_output(&merge_path, &["status", "--porcelain"])
            .trim()
            .is_empty(),
        "the aborted merge leaves a clean work tree"
    );
    merge_terminal.screenshot("interrupted_state/alt-x-aborted-merge");

    let rebase_path = repository_stopped_in_a_rebase("alt-x-rebase");
    let mut rebase_terminal = interrupted_terminal(&rebase_path);
    rebase_terminal.send_keys("alt+x");
    rebase_terminal.send_keys("tab,enter");
    rebase_terminal.tick();
    rebase_terminal.tick();
    assert_eq!(
        rebase_terminal.driver().status_message(),
        Some((REBASE_ABORTED_RECEIPT, StatusTone::Info)),
        "the status bar reports the aborted rebase"
    );
    assert!(
        !rebase_terminal.screen_text().contains(REBASING_LABEL),
        "the rebasing marker disappears, screen was:\n{}",
        rebase_terminal.screen_text()
    );
    rebase_terminal.screenshot("interrupted_state/alt-x-aborted-rebase");
}

#[test]
fn aborting_a_merge_with_a_dirty_worktree_shows_the_operation_failed_block() {
    let repository_path = repository_stopped_in_a_merge("alt-x-dirty");
    fs::write(
        repository_path.join(CLEAN_PATH),
        "fn clean() {\n    local();\n}\n",
    )
    .expect("the local edit is written");
    let mut terminal = interrupted_terminal(&repository_path);
    terminal.send_keys("alt+x");
    terminal.send_keys("tab,enter");
    terminal.tick();
    terminal.tick();
    let (source, message) = terminal.driver().panel_error().unwrap_or_else(|| {
        panic!(
            "the failed abort is reported, screen was:\n{}",
            terminal.screen_text()
        )
    });
    assert_eq!(source, PanelErrorSource::Operation);
    assert!(
        message.contains("merge --abort failed"),
        "the block keeps the git reason: {message}"
    );
    assert!(
        terminal.screen_text().contains("Operation failed"),
        "the block is titled Operation failed, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("interrupted_state/alt-x-abort-failed");
}
