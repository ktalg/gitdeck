use std::path::PathBuf;

use gitdeck_app::App;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    push_dialog_layout, push_message_text, remote_tool_bar_rects, PushDialogButton,
    PushDialogFocus, PushDialogLayout, PUSH_CANCEL_BUTTON_LABEL, PUSH_CONFIRM_BUTTON_LABEL,
    PUSH_FORCE_WITH_LEASE_LABEL, PUSH_SET_UPSTREAM_LABEL, REMOTE_PUSH_TOOL_LABEL,
    REMOTE_TOOL_BAR_LABELS,
};
use ratatui::layout::Rect;

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/push-dialog";
const WIDE_WIDTH: u16 = 220;
const WIDE_HEIGHT: u16 = 70;
const COMPACT_WIDTH: u16 = 80;
const COMPACT_HEIGHT: u16 = 24;
const CHECKED_OPTION_PREFIX: &str = "[x]";
const UNCHECKED_OPTION_PREFIX: &str = "[ ]";

fn push_repository(name: &str) -> PathBuf {
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    copy_directory(&fixture_repo(FixtureRepo::Dirty), &target)
        .expect("the dirty fixture repository is copied");
    target
}

fn push_terminal(case: &str, width: u16, height: u16) -> TestTerminal<App> {
    let repository_path = push_repository(case);
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

fn dialog_layout(width: u16, height: u16) -> PushDialogLayout {
    push_dialog_layout(Rect::new(0, 0, width, height)).expect("the screen holds the push dialog")
}

fn row_text(terminal: &TestTerminal<App>, row: Rect) -> String {
    (row.x..row.right())
        .map(|column| terminal.cell(column, row.y).symbol)
        .collect()
}

fn push_label_column(terminal: &TestTerminal<App>) -> u16 {
    let app = terminal.driver();
    let rects = remote_tool_bar_rects(
        app.layout().top_bar,
        app.project_name().as_deref(),
        app.head_info().branch.as_deref(),
        app.head_info().ahead,
        app.head_info().behind,
        app.caps(),
    );
    let push_rect = rects
        .into_iter()
        .zip(REMOTE_TOOL_BAR_LABELS)
        .find(|(_, label)| *label == REMOTE_PUSH_TOOL_LABEL)
        .map(|(rect, _)| rect)
        .expect("the push label keeps its own rect");
    assert!(push_rect.width > 0, "the push label stays on the top bar");
    push_rect.x
}

fn open_push_dialog(terminal: &mut TestTerminal<App>) {
    let column = push_label_column(terminal);
    let row = terminal.driver().layout().top_bar.y;
    terminal.mouse_click(column, row);
    assert!(
        terminal.driver().push_dialog_visible(),
        "clicking the top bar push label opens the dialog, screen was:\n{}",
        terminal.screen_text()
    );
}

#[test]
fn push_dialog_renders_over_the_shell() {
    for (width, height, case) in [
        (WIDE_WIDTH, WIDE_HEIGHT, "wide"),
        (COMPACT_WIDTH, COMPACT_HEIGHT, "compact"),
    ] {
        let mut terminal = push_terminal(&format!("render-{case}"), width, height);
        open_push_dialog(&mut terminal);
        let layout = dialog_layout(width, height);
        assert!(
            layout.frame.right() <= width && layout.frame.bottom() <= height,
            "{case} the dialog stays inside the {width}x{height} screen, frame {:?}",
            layout.frame
        );
        assert_eq!(
            row_text(&terminal, layout.message_row).trim_end(),
            push_message_text("main"),
            "{case} the message row names the branch and the remote"
        );
        assert!(
            row_text(&terminal, layout.set_upstream_row).contains(PUSH_SET_UPSTREAM_LABEL),
            "{case} the upstream option is on its own row"
        );
        assert!(
            row_text(&terminal, layout.force_with_lease_row).contains(PUSH_FORCE_WITH_LEASE_LABEL),
            "{case} the lease option is on its own row"
        );
        let button_row = row_text(&terminal, layout.button_row);
        assert!(
            button_row.contains(PUSH_CONFIRM_BUTTON_LABEL),
            "{case} the confirm button sits in the button row, row {button_row:?}"
        );
        assert!(
            button_row.contains(PUSH_CANCEL_BUTTON_LABEL),
            "{case} the cancel button sits in the button row, row {button_row:?}"
        );
        let title_row = terminal.screen_lines()[usize::from(layout.frame.y)].clone();
        assert!(
            title_row.contains("Push"),
            "{case} the dialog keeps its title, row {title_row:?}"
        );
        assert!(
            !terminal.screen_text().contains("force (no lease)"),
            "{case} the dialog offers no bare force option"
        );
        terminal.screenshot(&format!("push_dialog/render-{case}"));
    }
}

#[test]
fn push_dialog_swallows_keys_while_open() {
    let mut terminal = push_terminal("swallow", WIDE_WIDTH, WIDE_HEIGHT);
    terminal.send_keys("ctrl+k");
    for _ in 0..3 {
        terminal.send_keys("j");
    }
    let selected_file = terminal
        .driver()
        .selected_file_path()
        .map(str::to_string)
        .expect("the changes list starts on a file");
    let selected_commit = terminal
        .driver()
        .selected_commit()
        .map(|commit| commit.id.clone())
        .expect("the log starts on a commit");
    open_push_dialog(&mut terminal);
    for key in ["j", "down", "k", "up", "page-down"] {
        terminal.send_keys(key);
    }
    assert_eq!(
        terminal.driver().selected_file_path(),
        Some(selected_file.as_str()),
        "the dialog keeps the changes selection still"
    );
    assert_eq!(
        terminal
            .driver()
            .selected_commit()
            .map(|commit| commit.id.clone()),
        Some(selected_commit),
        "the dialog keeps the log selection still"
    );
    assert!(
        terminal.driver().file_diff().is_none(),
        "the dialog keeps enter away from the diff viewer"
    );
    assert!(
        terminal.driver().push_dialog_visible(),
        "the dialog stays open while it swallows keys"
    );
    terminal.screenshot("push_dialog/swallow");
}

#[test]
fn space_toggles_the_upstream_option() {
    let mut terminal = push_terminal("upstream", WIDE_WIDTH, WIDE_HEIGHT);
    open_push_dialog(&mut terminal);
    let layout = dialog_layout(WIDE_WIDTH, WIDE_HEIGHT);
    assert_eq!(
        terminal.driver().push_dialog_focus(),
        PushDialogFocus::SetUpstream
    );
    terminal.send_keys("space");
    assert!(terminal.driver().push_dialog_set_upstream());
    assert!(
        row_text(&terminal, layout.set_upstream_row).starts_with(CHECKED_OPTION_PREFIX),
        "the checked upstream row shows a checked box"
    );
    terminal.screenshot("push_dialog/upstream-checked");
    terminal.send_keys("space");
    assert!(!terminal.driver().push_dialog_set_upstream());
    assert!(
        row_text(&terminal, layout.set_upstream_row).starts_with(UNCHECKED_OPTION_PREFIX),
        "the unchecked upstream row shows an empty box"
    );
    terminal.send_keys("tab");
    assert_eq!(
        terminal.driver().push_dialog_focus(),
        PushDialogFocus::ForceWithLease
    );
    terminal.send_keys("space");
    assert!(terminal.driver().push_dialog_force_with_lease());
    assert!(
        row_text(&terminal, layout.force_with_lease_row).starts_with(CHECKED_OPTION_PREFIX),
        "the checked lease row shows a checked box"
    );
    assert!(
        !terminal.driver().push_dialog_set_upstream(),
        "the lease toggle leaves the upstream option alone"
    );
}

#[test]
fn esc_closes_the_push_dialog_without_pushing() {
    let mut terminal = push_terminal("escape", WIDE_WIDTH, WIDE_HEIGHT);
    open_push_dialog(&mut terminal);
    terminal.send_keys("esc");
    assert!(
        !terminal.driver().push_dialog_visible(),
        "escape closes the dialog"
    );
    assert!(
        !terminal.screen_text().contains(PUSH_FORCE_WITH_LEASE_LABEL),
        "the closed dialog leaves no rows behind, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(
        terminal.driver().panel_error().is_none(),
        "escape never reaches the push call"
    );
    assert_eq!(
        terminal.driver().status_message(),
        None,
        "escape leaves no push receipt"
    );
}

#[test]
fn enter_on_the_cancel_button_closes_without_pushing() {
    let mut terminal = push_terminal("cancel", WIDE_WIDTH, WIDE_HEIGHT);
    open_push_dialog(&mut terminal);
    for _ in 0..2 {
        terminal.send_keys("tab");
    }
    assert_eq!(
        terminal.driver().push_dialog_focus(),
        PushDialogFocus::Buttons
    );
    terminal.send_keys("right");
    assert_eq!(
        terminal.driver().push_dialog_button(),
        PushDialogButton::Cancel
    );
    terminal.send_keys("enter");
    assert!(
        !terminal.driver().push_dialog_visible(),
        "enter on the cancel button closes the dialog"
    );
    assert!(
        terminal.driver().panel_error().is_none(),
        "the cancel button never reaches the push call"
    );
    assert_eq!(
        terminal.driver().status_message(),
        None,
        "the cancel button leaves no push receipt"
    );
}
