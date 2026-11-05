use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::App;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    discard_dialog_layout, discard_message_text, DiscardDialogButton, DiscardDialogLayout,
    DISCARD_CANCEL_BUTTON_LABEL, DISCARD_CONFIRM_BUTTON_LABEL, DISCARD_IRREVERSIBLE_LABEL,
};
use ratatui::layout::Rect;

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/discard-changes";
const WIDE_WIDTH: u16 = 160;
const WIDE_HEIGHT: u16 = 48;
const COMPACT_WIDTH: u16 = 80;
const COMPACT_HEIGHT: u16 = 24;
const CHANGES_ROW_SCAN_LIMIT: usize = 32;
const MODIFIED_PATH: &str = "src/app.rs";
const HEAD_FILE_LINE: &str = "run_twice";
const DISCARDED_FILE_LINE: &str = "shutdown";

fn discard_repository(name: &str) -> PathBuf {
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    copy_directory(&fixture_repo(FixtureRepo::Dirty), &target)
        .expect("the dirty fixture repository is copied");
    target
}

fn discard_terminal(name: &str, width: u16, height: u16) -> TestTerminal<App> {
    let repository_path = discard_repository(name);
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        width,
        height,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(width, height);
    terminal
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
        terminal.send_keys("j");
    }
    panic!(
        "the changes pane never selected {path}, the cursor holds {:?}",
        terminal.driver().selected_file_path()
    );
}

fn dialog_layout(width: u16, height: u16) -> DiscardDialogLayout {
    discard_dialog_layout(Rect::new(0, 0, width, height))
        .expect("the screen holds the discard dialog")
}

fn row_text(terminal: &TestTerminal<App>, row: Rect) -> String {
    (row.x..row.right())
        .map(|column| terminal.cell(column, row.y).symbol)
        .collect()
}

fn changes_hold_path(terminal: &TestTerminal<App>, path: &str) -> bool {
    terminal
        .driver()
        .changes()
        .iter()
        .any(|change| change.path == path)
}

fn status_text(terminal: &TestTerminal<App>) -> String {
    terminal
        .driver()
        .status_message()
        .map(|(text, _)| text.to_string())
        .unwrap_or_default()
}

fn file_text(repository_path: &Path, path: &str) -> String {
    fs::read_to_string(repository_path.join(path)).expect("the work tree file is readable")
}

#[test]
fn alt_d_opens_a_confirmation_with_the_cancel_button_focused() {
    for (width, height, case) in [
        (WIDE_WIDTH, WIDE_HEIGHT, "wide"),
        (COMPACT_WIDTH, COMPACT_HEIGHT, "compact"),
    ] {
        let mut terminal = discard_terminal(&format!("confirm-{case}"), width, height);
        select_file(&mut terminal, MODIFIED_PATH);
        terminal.send_keys("alt+d");
        assert!(
            terminal.driver().discard_dialog_visible(),
            "{case} alt+d opens the confirmation, screen was:\n{}",
            terminal.screen_text()
        );
        assert_eq!(
            terminal.driver().discard_dialog_path(),
            Some(MODIFIED_PATH),
            "{case} the confirmation names the selected file"
        );
        assert_eq!(
            terminal.driver().discard_dialog_button(),
            DiscardDialogButton::Cancel,
            "{case} the confirmation starts on Cancel"
        );
        let layout = dialog_layout(width, height);
        assert_eq!(
            row_text(&terminal, layout.message_row).trim_end(),
            discard_message_text(MODIFIED_PATH),
            "{case} the message row names the file"
        );
        assert_eq!(
            row_text(&terminal, layout.irreversible_row).trim_end(),
            DISCARD_IRREVERSIBLE_LABEL,
            "{case} the second row warns that the action cannot be undone"
        );
        let button_row = row_text(&terminal, layout.button_row);
        assert!(
            button_row.contains(DISCARD_CONFIRM_BUTTON_LABEL),
            "{case} the discard button sits in the button row, row {button_row:?}"
        );
        assert!(
            button_row.contains(DISCARD_CANCEL_BUTTON_LABEL),
            "{case} the cancel button sits in the button row, row {button_row:?}"
        );
        let title_row = terminal.screen_lines()[usize::from(layout.frame.y)].clone();
        assert!(
            title_row.contains("Discard Changes"),
            "{case} the dialog keeps its title, row {title_row:?}"
        );
        terminal.screenshot(&format!("discard_changes/confirm-{case}"));
    }
}

#[test]
fn enter_on_cancel_keeps_the_file_and_reports_it() {
    let mut terminal = discard_terminal("cancel", WIDE_WIDTH, WIDE_HEIGHT);
    let repository_path = terminal
        .driver()
        .repository_path()
        .expect("the scratch repository is open")
        .to_path_buf();
    let original_text = file_text(&repository_path, MODIFIED_PATH);
    select_file(&mut terminal, MODIFIED_PATH);
    terminal.send_keys("alt+d");
    assert!(terminal.driver().discard_dialog_visible());
    terminal.send_keys("enter");
    assert!(
        !terminal.driver().discard_dialog_visible(),
        "the focused Cancel button closes the confirmation"
    );
    assert_eq!(
        file_text(&repository_path, MODIFIED_PATH),
        original_text,
        "the cancelled confirmation leaves the work tree file alone"
    );
    assert!(
        changes_hold_path(&terminal, MODIFIED_PATH),
        "the cancelled confirmation leaves the file in the change list"
    );
    assert!(
        terminal.driver().panel_error().is_none(),
        "the cancelled confirmation reports no error"
    );
    assert_eq!(
        status_text(&terminal),
        format!("{MODIFIED_PATH} kept"),
        "the cancelled confirmation reports that the file stays"
    );
    terminal.screenshot("discard_changes/cancelled");
}

#[test]
fn confirming_the_discard_removes_the_change() {
    let mut terminal = discard_terminal("confirm", WIDE_WIDTH, WIDE_HEIGHT);
    let repository_path = terminal
        .driver()
        .repository_path()
        .expect("the scratch repository is open")
        .to_path_buf();
    select_file(&mut terminal, MODIFIED_PATH);
    terminal.send_keys("alt+d");
    assert!(terminal.driver().discard_dialog_visible());
    terminal.send_keys("tab");
    assert_eq!(
        terminal.driver().discard_dialog_button(),
        DiscardDialogButton::Discard,
        "tab moves the focus to the discard button"
    );
    terminal.send_keys("enter");
    assert!(
        !terminal.driver().discard_dialog_visible(),
        "the confirmation closes once the discard runs"
    );
    let restored_text = file_text(&repository_path, MODIFIED_PATH);
    assert!(
        restored_text.contains(HEAD_FILE_LINE) && !restored_text.contains(DISCARDED_FILE_LINE),
        "the discarded file holds the head version again, file was {restored_text:?}"
    );
    assert!(
        !changes_hold_path(&terminal, MODIFIED_PATH),
        "the discarded file leaves the change list"
    );
    let status_text = status_text(&terminal);
    assert!(
        status_text.contains(&format!("Discarded changes to {MODIFIED_PATH}")),
        "the status bar reports the discard, status was {status_text:?}"
    );
    terminal.screenshot("discard_changes/confirmed");
}
