use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::App;
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    changes_panel_layout, panel_error_layout, PanelErrorLayout, LOCKED_INDEX_HINT,
    NEXT_STEP_HINT_PREFIX,
};

const REFERENCE_WIDTH: u16 = 120;
const REFERENCE_HEIGHT: u16 = 32;
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const LONG_HOOK_MESSAGE: &str = "the pre-commit hook refused this change because the staged files are missing the required header and the licence header of every new file must name the copyright holder and the year of the first publication and the reviewer list of the change must name at least two reviewers of the owning team and the changelog entry must link the ticket of the change and the reviewer list has not been filled in yet";
const SCROLL_INDICATOR: &str = "↑↓";
const OVERFLOW_MARKER: &str = "(+";
const LOCKED_INDEX_MESSAGE: &str = "fatal: Unable to create '/repo/.git/index.lock': File exists.";

fn scratch_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, &["init", "-q", "-b", "main"]);
    fs::write(path.join("tracked.txt"), "tracked\n").expect("the tracked file is written");
    run_git(&path, &["add", "tracked.txt"]);
    run_git(
        &path,
        &[
            "-c",
            "user.name=gitdeck-suite",
            "-c",
            "user.email=suite@gitdeck.invalid",
            "commit",
            "-q",
            "-m",
            "base commit",
        ],
    );
    fs::write(path.join("tracked.txt"), "tracked change\n").expect("the edit is written");
    path
}

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

fn install_failing_commit_hook(repository_path: &Path) {
    let hook_path = repository_path
        .join(".git")
        .join("hooks")
        .join("pre-commit");
    fs::write(
        &hook_path,
        format!("#!/bin/sh\nprintf '{LONG_HOOK_MESSAGE}\\n' >&2\nexit 1\n"),
    )
    .expect("the failing pre-commit hook is written");
    grant_execute_permission(&hook_path);
}

#[cfg(unix)]
fn grant_execute_permission(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .expect("the hook file exists")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("the hook file becomes executable");
}

#[cfg(not(unix))]
fn grant_execute_permission(_path: &Path) {}

fn changes_list_rows(terminal: &TestTerminal<App>) -> Rect {
    let changes = terminal
        .pane_rect(PaneId::Changes)
        .expect("the changes pane is visible");
    changes_panel_layout(changes, false)
        .expect("the changes panel holds a commit block")
        .list
}

fn panel_error_body(terminal: &TestTerminal<App>) -> PanelErrorLayout {
    panel_error_layout(
        changes_list_rows(terminal),
        Rect::new(0, 0, REFERENCE_WIDTH, REFERENCE_HEIGHT),
    )
    .expect("the changes list holds the error block")
}

fn body_row_text(terminal: &TestTerminal<App>, row: u16) -> String {
    let body = panel_error_body(terminal).body;
    let mut text = String::new();
    for column in body.x..body.right() {
        text.push_str(&terminal.cell(column, row).symbol);
    }
    text.trim_end().to_string()
}

fn body_rows_text(terminal: &TestTerminal<App>) -> Vec<String> {
    let body = panel_error_body(terminal).body;
    (body.y..body.bottom())
        .map(|row| body_row_text(terminal, row))
        .collect()
}

fn terminal_holding_a_long_commit_failure(name: &str) -> TestTerminal<App> {
    let repository_path = scratch_repository(name);
    install_failing_commit_hook(&repository_path);
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    terminal.driver_mut().set_commit_message("doomed commit");
    terminal.send_keys("ctrl+enter");
    assert!(
        terminal.driver().panel_error().is_some(),
        "the failing commit records a panel error"
    );
    terminal
}

#[test]
fn error_block_scrolls_when_the_body_overflows() {
    let mut terminal = terminal_holding_a_long_commit_failure("error-readability-scroll");
    let first_window = body_rows_text(&terminal);
    assert!(
        first_window.iter().any(|row| row.contains(OVERFLOW_MARKER)),
        "the overflowing body reports the hidden lines, rows {first_window:?}"
    );
    assert!(
        terminal.screen_text().contains(SCROLL_INDICATOR),
        "the overflowing body advertises the scroll keys, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.send_keys("down");
    let scrolled_window = body_rows_text(&terminal);
    assert_ne!(
        first_window, scrolled_window,
        "the down key scrolls the error body"
    );
    assert!(
        terminal.driver().panel_error().is_some(),
        "scrolling keeps the error block open"
    );
    terminal.send_keys("up");
    assert_eq!(
        first_window,
        body_rows_text(&terminal),
        "the up key scrolls the error body back"
    );
    terminal.screenshot("error_readability/error-block-scrolled");
}

#[test]
fn scrolling_to_the_end_drops_the_overflow_marker() {
    let mut terminal = terminal_holding_a_long_commit_failure("error-readability-scroll-end");
    for _ in 0..12 {
        terminal.send_keys("down");
    }
    let last_window = body_rows_text(&terminal);
    assert!(
        !last_window.iter().any(|row| row.contains(OVERFLOW_MARKER)),
        "the last window holds no marker, rows {last_window:?}"
    );
    assert!(
        last_window.iter().any(|row| row.contains("filled in yet")),
        "the last window shows the tail of the reason, rows {last_window:?}"
    );
    assert!(
        terminal.driver().panel_error().is_some(),
        "the error block stays open at the end of the body"
    );
}

fn terminal_holding_a_wide_commit_failure(
    name: &str,
    width: u16,
    height: u16,
) -> TestTerminal<App> {
    let repository_path = scratch_repository(name);
    let hook_path = repository_path
        .join(".git")
        .join("hooks")
        .join("pre-commit");
    fs::write(
        &hook_path,
        format!("#!/bin/sh\nprintf '{LOCKED_INDEX_MESSAGE}\\n' >&2\nexit 1\n"),
    )
    .expect("the failing pre-commit hook is written");
    grant_execute_permission(&hook_path);
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        width,
        height,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(width, height);
    terminal.driver_mut().set_commit_message("doomed commit");
    terminal.send_keys("ctrl+enter");
    assert!(
        terminal.driver().panel_error().is_some(),
        "the failing commit records a panel error"
    );
    terminal
}

fn screen_row_text(terminal: &TestTerminal<App>, row: u16, width: u16) -> String {
    (0..width)
        .map(|column| terminal.cell(column, row).symbol)
        .collect::<String>()
        .trim_end()
        .to_string()
}

#[test]
fn the_error_dialog_uses_the_screen_width() {
    let wide_width = 160;
    let wide_height = 48;
    let wide = terminal_holding_a_wide_commit_failure("error-width-wide", wide_width, wide_height);
    let wide_layout = panel_error_layout(
        changes_list_rows(&wide),
        Rect::new(0, 0, wide_width, wide_height),
    )
    .expect("the wide screen holds the error block");
    assert!(
        wide_layout.block.width >= 60,
        "the error block grows with the screen, width was {}",
        wide_layout.block.width
    );
    let hint_row = wide_layout.body.bottom().saturating_sub(1);
    let hint_text = screen_row_text(&wide, hint_row, wide_width);
    assert!(
        hint_text.contains(NEXT_STEP_HINT_PREFIX),
        "the hint row carries its prefix, row was {hint_text:?}"
    );
    assert!(
        hint_text
            .replace(' ', "")
            .contains(&LOCKED_INDEX_HINT.replace(' ', "")),
        "the hint row is complete at 160 columns, row was {hint_text:?}"
    );
    wide.screenshot("error_readability/wide-error-dialog");

    let compact_width = 80;
    let compact_height = 24;
    let compact = terminal_holding_a_wide_commit_failure(
        "error-width-compact",
        compact_width,
        compact_height,
    );
    let compact_layout = panel_error_layout(
        changes_list_rows(&compact),
        Rect::new(0, 0, compact_width, compact_height),
    )
    .expect("the compact screen holds the error block");
    assert!(
        compact_layout.block.right() <= compact_width,
        "the compact block stays inside the screen, block was {:?}",
        compact_layout.block
    );
    assert!(
        compact_layout.block.bottom() <= compact_height,
        "the compact block stays above the bottom edge, block was {:?}",
        compact_layout.block
    );
    let compact_hint_row = compact_layout.body.bottom().saturating_sub(1);
    let compact_hint = screen_row_text(&compact, compact_hint_row, compact_width);
    assert!(
        compact_hint
            .replace(' ', "")
            .contains(&LOCKED_INDEX_HINT.replace(' ', "")),
        "the hint row is complete at 80 columns, row was {compact_hint:?}"
    );
    compact.screenshot("error_readability/compact-error-dialog");
}
