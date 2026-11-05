use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::App;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    delete_branch_dialog_layout, delete_branch_message_text, DeleteBranchDialogButton,
    DELETE_BRANCH_CANCEL_BUTTON_LABEL, DELETE_BRANCH_CONFIRM_BUTTON_LABEL,
    DELETE_BRANCH_DIALOG_TITLE, DISCARD_IRREVERSIBLE_LABEL,
};
use ratatui::layout::Rect;

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/branch-housekeeping";
const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const CURRENT_BRANCH: &str = "main";
const MERGED_BRANCH: &str = "feature/merged";
const BASE_PATH: &str = "src/base.txt";
const MERGED_PATH: &str = "src/merged.txt";
const UNTRACKED_PATH: &str = "notes.txt";
const IGNORED_RECEIPT: &str = "Ignored notes.txt";
const DELETED_RECEIPT: &str = "Deleted feature/merged";
const CHANGES_ROW_SCAN_LIMIT: usize = 32;

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

fn write_file(repository_path: &Path, relative_path: &str, contents: &str) {
    let path = repository_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the parent directory is created");
    }
    fs::write(path, contents).expect("the scratch file is written");
}

fn scratch_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, &["init", "-q", "-b", CURRENT_BRANCH]);
    write_file(&path, BASE_PATH, "base\n");
    run_git(&path, &["add", "--all"]);
    run_git(&path, &["commit", "-q", "-m", "base commit"]);
    run_git(&path, &["switch", "-q", "-c", MERGED_BRANCH]);
    write_file(&path, MERGED_PATH, "merged\n");
    run_git(&path, &["add", "--all"]);
    run_git(&path, &["commit", "-q", "-m", "merged commit"]);
    run_git(&path, &["switch", "-q", CURRENT_BRANCH]);
    run_git(&path, &["merge", "-q", "--no-ff", MERGED_BRANCH]);
    path
}

fn housekeeping_terminal(repository_path: &Path) -> TestTerminal<App> {
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

fn branch_names(repository_path: &Path) -> Vec<String> {
    run_git(repository_path, &["branch", "--format=%(refname:short)"])
        .lines()
        .map(str::to_string)
        .collect()
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

fn select_file(terminal: &mut TestTerminal<App>, path: &str) {
    terminal.send_keys("ctrl+k");
    for _ in 0..CHANGES_ROW_SCAN_LIMIT {
        terminal.send_keys("k");
    }
    for _ in 0..CHANGES_ROW_SCAN_LIMIT {
        if terminal.driver().selected_file_path() == Some(path) {
            return;
        }
        terminal.send_keys("right");
        terminal.send_keys("j");
    }
    panic!(
        "the changes pane never selected {path}, the cursor holds {:?}",
        terminal.driver().selected_file_path()
    );
}

fn row_text(terminal: &TestTerminal<App>, row: Rect) -> String {
    (row.x..row.right())
        .map(|column| terminal.cell(column, row.y).symbol)
        .collect()
}

#[test]
fn alt_shift_d_opens_the_delete_confirmation_with_cancel_focused() {
    let repository_path = scratch_repository("delete-confirm");
    let mut terminal = housekeeping_terminal(&repository_path);
    select_the_branch(&mut terminal, MERGED_BRANCH);
    terminal.send_keys("alt+shift+d");
    terminal.tick();
    assert!(
        terminal.driver().delete_branch_dialog_visible(),
        "alt+shift+d opens the confirmation, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.driver().delete_branch_dialog_name(),
        Some(MERGED_BRANCH)
    );
    assert_eq!(
        terminal.driver().delete_branch_dialog_button(),
        DeleteBranchDialogButton::Cancel,
        "the confirmation starts on Cancel"
    );
    let layout = delete_branch_dialog_layout(Rect::new(0, 0, MEDIUM_WIDTH, MEDIUM_HEIGHT))
        .expect("the screen holds the delete dialog");
    let title_row = Rect::new(layout.frame.x, layout.frame.y, layout.frame.width, 1);
    assert!(row_text(&terminal, title_row).contains(DELETE_BRANCH_DIALOG_TITLE));
    assert_eq!(
        row_text(&terminal, layout.message_row).trim_end(),
        delete_branch_message_text(MERGED_BRANCH)
    );
    assert_eq!(
        row_text(&terminal, layout.irreversible_row).trim_end(),
        DISCARD_IRREVERSIBLE_LABEL
    );
    let button_text = row_text(&terminal, layout.button_row);
    assert!(button_text.contains(DELETE_BRANCH_CONFIRM_BUTTON_LABEL));
    assert!(button_text.contains(DELETE_BRANCH_CANCEL_BUTTON_LABEL));
    terminal.screenshot("branch_housekeeping/alt-shift-d-confirm");
}

#[test]
fn cancelling_the_delete_keeps_the_branch() {
    let repository_path = scratch_repository("delete-cancelled");
    let mut terminal = housekeeping_terminal(&repository_path);
    select_the_branch(&mut terminal, MERGED_BRANCH);
    terminal.send_keys("alt+shift+d");
    terminal.tick();
    terminal.send_keys("enter");
    terminal.tick();
    assert!(
        !terminal.driver().delete_branch_dialog_visible(),
        "enter on Cancel closes the confirmation"
    );
    assert_eq!(terminal.driver().status_message(), None);
    assert!(
        branch_names(&repository_path)
            .iter()
            .any(|name| name == MERGED_BRANCH),
        "a cancelled confirmation keeps the branch"
    );
    terminal.screenshot("branch_housekeeping/delete-cancelled");
}

#[test]
fn confirming_the_delete_removes_the_branch_and_reports_it() {
    let repository_path = scratch_repository("delete-confirmed");
    let mut terminal = housekeeping_terminal(&repository_path);
    select_the_branch(&mut terminal, MERGED_BRANCH);
    terminal.send_keys("alt+shift+d");
    terminal.tick();
    terminal.send_keys("tab");
    terminal.tick();
    assert_eq!(
        terminal.driver().delete_branch_dialog_button(),
        DeleteBranchDialogButton::Delete
    );
    terminal.send_keys("enter");
    terminal.tick();
    assert_eq!(
        terminal.driver().status_message().map(|(text, _)| text),
        Some(DELETED_RECEIPT)
    );
    assert!(
        !branch_names(&repository_path)
            .iter()
            .any(|name| name == MERGED_BRANCH),
        "the deleted branch leaves the branch list"
    );
    assert!(
        !terminal
            .driver()
            .branches()
            .iter()
            .any(|branch| branch.name == MERGED_BRANCH),
        "the branch list no longer holds the deleted branch"
    );
    terminal.screenshot("branch_housekeeping/delete-confirmed");
}

#[test]
fn alt_i_ignores_the_selected_untracked_file() {
    let repository_path = scratch_repository("ignore-untracked");
    write_file(&repository_path, UNTRACKED_PATH, "scratch\n");
    let mut terminal = housekeeping_terminal(&repository_path);
    select_file(&mut terminal, UNTRACKED_PATH);
    terminal.send_keys("alt+i");
    terminal.tick();
    assert_eq!(
        terminal.driver().status_message().map(|(text, _)| text),
        Some(IGNORED_RECEIPT)
    );
    let gitignore =
        fs::read_to_string(repository_path.join(".gitignore")).expect("the ignore file is written");
    assert_eq!(gitignore, format!("{UNTRACKED_PATH}\n"));
    assert!(
        !terminal
            .driver()
            .changes()
            .iter()
            .any(|change| change.path == UNTRACKED_PATH),
        "the ignored file leaves the changes list"
    );
    let screen_text = terminal.screen_text();
    assert!(
        screen_text.contains(IGNORED_RECEIPT),
        "the footer reports the ignore, screen was:\n{screen_text}"
    );
    terminal.screenshot("branch_housekeeping/alt-i-ignored");
}

#[test]
fn alt_i_on_a_tracked_file_is_a_no_op() {
    let repository_path = scratch_repository("ignore-tracked");
    write_file(&repository_path, BASE_PATH, "edited\n");
    let mut terminal = housekeeping_terminal(&repository_path);
    select_file(&mut terminal, BASE_PATH);
    terminal.send_keys("alt+i");
    terminal.tick();
    assert_eq!(terminal.driver().status_message(), None);
    assert_eq!(terminal.driver().panel_error(), None);
    assert!(
        !repository_path.join(".gitignore").exists(),
        "a tracked file writes no ignore entry"
    );
}
