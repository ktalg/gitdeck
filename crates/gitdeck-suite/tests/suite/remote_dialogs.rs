use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::{App, STATUS_MESSAGE_LIFETIME_TICKS};
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    fetch_dialog_layout, pull_dialog_layout, pull_message_text, remote_tool_bar_rects,
    FetchDialogLayout, PanelErrorSource, PullDialogLayout, PullStrategyOption, StatusTone,
    FETCH_NO_REMOTE_LABEL, PULL_CHECKED_MARKER, PULL_UNCHECKED_MARKER, REMOTE_FETCH_TOOL_LABEL,
    REMOTE_PULL_TOOL_LABEL, REMOTE_PUSH_TOOL_LABEL, REMOTE_TOOL_BAR_LABELS,
};
use ratatui::layout::Rect;
use ratatui::style::Modifier;

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/remote-dialogs";
const WIDE_WIDTH: u16 = 220;
const WIDE_HEIGHT: u16 = 70;
const CURRENT_BRANCH: &str = "main";
const PULL_RECEIPT: &str = "Pulled main from origin";
const PUSH_RECEIPT: &str = "Pushed main to origin";
const FETCH_RECEIPT: &str = "Fetched from origin";

fn scratch_path(name: &str) -> PathBuf {
    repository_root()
        .join(SCRATCH_REPOSITORY_DIRECTORY)
        .join(name)
}

fn git_output(repository_path: &Path, arguments: &[&str]) -> String {
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

fn dirty_repository(name: &str) -> PathBuf {
    let target = scratch_path(name);
    if target.exists() {
        fs::remove_dir_all(&target).expect("the stale scratch repository is removed");
    }
    copy_directory(&fixture_repo(FixtureRepo::Dirty), &target)
        .expect("the dirty fixture repository is copied");
    target
}

fn published_repository(name: &str) -> PathBuf {
    let repository = dirty_repository(name);
    let remote = scratch_path(&format!("{name}.git"));
    if remote.exists() {
        fs::remove_dir_all(&remote).expect("the stale bare remote is removed");
    }
    fs::create_dir_all(&remote).expect("the bare remote directory is created");
    git_output(&remote, &["init", "--bare", "--quiet"]);
    git_output(
        &repository,
        &[
            "remote",
            "add",
            "origin",
            remote
                .to_str()
                .expect("the bare remote path is valid utf-8"),
        ],
    );
    git_output(&repository, &["add", "-A"]);
    git_output(
        &repository,
        &["commit", "--quiet", "-m", "publish the fixture"],
    );
    git_output(
        &repository,
        &[
            "push",
            "--quiet",
            "--set-upstream",
            "origin",
            CURRENT_BRANCH,
        ],
    );
    repository
}

fn remote_side_copy(name: &str, source: &Path) -> PathBuf {
    let target = scratch_path(&format!("{name}-remote-side"));
    if target.exists() {
        fs::remove_dir_all(&target).expect("the stale remote side copy is removed");
    }
    copy_directory(source, &target).expect("the published repository is copied");
    target
}

fn advance_the_remote(remote_side: &Path, subject: &str) {
    git_output(
        remote_side,
        &["commit", "--allow-empty", "--quiet", "-m", subject],
    );
    git_output(remote_side, &["push", "--quiet", "origin", CURRENT_BRANCH]);
}

fn terminal_for(repository_path: &Path) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(
        App::with_repository(repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        WIDE_WIDTH,
        WIDE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(WIDE_WIDTH, WIDE_HEIGHT);
    terminal
}

fn label_column(terminal: &TestTerminal<App>, label: &str) -> u16 {
    let app = terminal.driver();
    let rects = remote_tool_bar_rects(
        app.layout().top_bar,
        app.project_name().as_deref(),
        app.head_info().branch.as_deref(),
        app.head_info().ahead,
        app.head_info().behind,
        app.caps(),
    );
    let rect = rects
        .into_iter()
        .zip(REMOTE_TOOL_BAR_LABELS)
        .find(|(_, bar_label)| *bar_label == label)
        .map(|(rect, _)| rect)
        .expect("the label keeps its own rect");
    assert!(rect.width > 0, "the {label} label stays on the top bar");
    rect.x
}

fn open_dialog(terminal: &mut TestTerminal<App>, label: &str) {
    let column = label_column(terminal, label);
    let row = terminal.driver().layout().top_bar.y;
    terminal.mouse_click(column, row);
}

fn pull_layout() -> PullDialogLayout {
    pull_dialog_layout(Rect::new(0, 0, WIDE_WIDTH, WIDE_HEIGHT))
        .expect("the screen holds the dialog")
}

fn fetch_layout() -> FetchDialogLayout {
    fetch_dialog_layout(Rect::new(0, 0, WIDE_WIDTH, WIDE_HEIGHT))
        .expect("the screen holds the dialog")
}

fn row_text(terminal: &TestTerminal<App>, row: Rect) -> String {
    (row.x..row.right())
        .map(|column| terminal.cell(column, row.y).symbol)
        .collect()
}

#[test]
fn pull_dialog_defaults_to_fast_forward_only() {
    let repository = dirty_repository("defaults");
    let mut terminal = terminal_for(&repository);
    open_dialog(&mut terminal, REMOTE_PULL_TOOL_LABEL);
    assert!(
        terminal.driver().pull_dialog_visible(),
        "clicking the top bar pull label opens the dialog, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.driver().pull_dialog_strategy(),
        PullStrategyOption::FastForwardOnly,
        "the dialog starts on the fast-forward strategy"
    );
    let layout = pull_layout();
    assert_eq!(
        row_text(&terminal, layout.message_row).trim_end(),
        pull_message_text(CURRENT_BRANCH)
    );
    assert!(
        row_text(&terminal, layout.fast_forward_row).starts_with(PULL_CHECKED_MARKER),
        "the fast-forward radio starts selected, row {:?}",
        row_text(&terminal, layout.fast_forward_row)
    );
    assert!(
        row_text(&terminal, layout.merge_row).starts_with(PULL_UNCHECKED_MARKER),
        "the merge radio starts unselected, row {:?}",
        row_text(&terminal, layout.merge_row)
    );
    assert!(
        row_text(&terminal, layout.rebase_row).starts_with(PULL_UNCHECKED_MARKER),
        "the rebase radio starts unselected, row {:?}",
        row_text(&terminal, layout.rebase_row)
    );
    terminal.screenshot("remote_dialogs/pull-defaults");
}

#[test]
fn pull_dialog_switches_the_strategy_with_the_keyboard() {
    let repository = published_repository("strategy");
    let remote_side = remote_side_copy("strategy", &repository);
    git_output(
        &repository,
        &["commit", "--allow-empty", "--quiet", "-m", "local work"],
    );
    advance_the_remote(&remote_side, "remote advance");
    let mut terminal = terminal_for(&repository);
    open_dialog(&mut terminal, REMOTE_PULL_TOOL_LABEL);
    terminal.send_keys("enter");
    match terminal.driver().panel_error() {
        Some((source, message)) => {
            assert_eq!(
                source,
                PanelErrorSource::Pull,
                "the failed fast-forward pull keeps its own title"
            );
            assert!(!message.is_empty(), "the failure carries a reason");
        }
        None => panic!(
            "the fast-forward pull refuses the diverged history, screen was:\n{}",
            terminal.screen_text()
        ),
    }
    terminal.send_keys("enter");
    assert!(terminal.driver().panel_error().is_none(), "enter dismisses");
    open_dialog(&mut terminal, REMOTE_PULL_TOOL_LABEL);
    terminal.send_keys("down");
    assert_eq!(
        terminal.driver().pull_dialog_strategy(),
        PullStrategyOption::Merge,
        "the down key moves the radio to the merge strategy"
    );
    let layout = pull_layout();
    assert!(
        row_text(&terminal, layout.merge_row).starts_with(PULL_CHECKED_MARKER),
        "the merge radio is selected, row {:?}",
        row_text(&terminal, layout.merge_row)
    );
    assert!(
        row_text(&terminal, layout.fast_forward_row).starts_with(PULL_UNCHECKED_MARKER),
        "the fast-forward radio is released, row {:?}",
        row_text(&terminal, layout.fast_forward_row)
    );
    terminal.send_keys("enter");
    assert!(
        terminal.driver().panel_error().is_none(),
        "the merge strategy reaches the backend and succeeds, screen was:\n{}",
        terminal.screen_text()
    );
    let status = terminal
        .driver()
        .status_message()
        .map(|(text, _)| text.to_string())
        .unwrap_or_default();
    assert_eq!(status, PULL_RECEIPT, "the merge pull reports its receipt");
    assert_eq!(
        git_output(&repository, &["log", "--format=%p", "-n", "1"])
            .split_whitespace()
            .count(),
        2,
        "the merge strategy leaves a two-parent commit behind"
    );
    terminal.screenshot("remote_dialogs/pull-merge");
}

#[test]
fn fetch_dialog_disables_the_button_without_a_remote() {
    let repository = dirty_repository("no-remote");
    let mut terminal = terminal_for(&repository);
    open_dialog(&mut terminal, REMOTE_FETCH_TOOL_LABEL);
    assert!(
        terminal.driver().fetch_dialog_visible(),
        "clicking the top bar fetch label opens the dialog, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(
        !terminal.driver().fetch_dialog_remote_available(),
        "the fixture has no remote to fetch from"
    );
    let layout = fetch_layout();
    assert_eq!(
        row_text(&terminal, layout.message_row).trim_end(),
        FETCH_NO_REMOTE_LABEL
    );
    let fetch_cell = terminal.cell(layout.fetch_button.x, layout.fetch_button.y);
    assert!(
        fetch_cell.modifier.contains(Modifier::DIM),
        "the fetch button keeps the disabled look, cell {fetch_cell:?}"
    );
    terminal.screenshot("remote_dialogs/fetch-no-remote");
    terminal.send_keys("enter");
    assert!(
        !terminal.driver().fetch_dialog_visible(),
        "enter on the disabled button closes the dialog"
    );
    match terminal.driver().panel_error() {
        Some((source, message)) => {
            assert_eq!(
                source,
                PanelErrorSource::Fetch,
                "the missing remote keeps the fetch title"
            );
            assert!(!message.is_empty(), "the failure carries a reason");
        }
        None => panic!(
            "the disabled fetch button still reports the missing remote, screen was:\n{}",
            terminal.screen_text()
        ),
    }
}

#[test]
fn dialogs_swallow_keys_while_open() {
    let repository = dirty_repository("swallow");
    let mut terminal = terminal_for(&repository);
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
    for label in [REMOTE_PULL_TOOL_LABEL, REMOTE_FETCH_TOOL_LABEL] {
        open_dialog(&mut terminal, label);
        for key in [
            "j",
            "k",
            "up",
            "down",
            "page-down",
            "page-up",
            "home",
            "end",
            "b",
        ] {
            terminal.send_keys(key);
        }
        assert_eq!(
            terminal.driver().selected_file_path(),
            Some(selected_file.as_str()),
            "the {label} dialog keeps the changes selection still"
        );
        assert_eq!(
            terminal
                .driver()
                .selected_commit()
                .map(|commit| commit.id.clone()),
            Some(selected_commit.clone()),
            "the {label} dialog keeps the log selection still"
        );
        assert!(
            terminal.driver().file_diff().is_none(),
            "the {label} dialog keeps enter away from the diff viewer"
        );
        let open =
            terminal.driver().pull_dialog_visible() || terminal.driver().fetch_dialog_visible();
        assert!(open, "the {label} dialog stays open while it swallows keys");
        terminal.screenshot(&format!("remote_dialogs/swallow-{label}"));
        terminal.send_keys("esc");
        let closed =
            !terminal.driver().pull_dialog_visible() && !terminal.driver().fetch_dialog_visible();
        assert!(closed, "escape closes the {label} dialog");
    }
}

#[test]
fn pull_without_a_remote_says_pull_not_push() {
    let repository = dirty_repository("pull-without-a-remote");
    let mut terminal = terminal_for(&repository);
    terminal.send_keys("p");
    terminal.tick();
    assert!(
        terminal.driver().pull_dialog_visible(),
        "the p key opens the pull dialog"
    );
    terminal.send_keys("enter");
    terminal.tick();
    let panel_error = terminal
        .driver()
        .panel_error()
        .expect("the pull without a remote records an error");
    assert_eq!(panel_error.1, "the repository has no remote to pull from");
    assert!(
        !panel_error.1.contains("push"),
        "the pull failure never talks about pushing, message was {:?}",
        panel_error.1
    );
    let screen_text = terminal.screen_text();
    assert!(
        screen_text.contains("Pull failed"),
        "the error block is titled Pull failed, screen was:\n{screen_text}"
    );
    assert!(
        screen_text.contains("no remote to pull from"),
        "the block explains the missing pull remote, screen was:\n{screen_text}"
    );
    terminal.screenshot("remote_dialogs/pull-no-remote");
}

#[test]
fn a_successful_push_reports_a_receipt() {
    let repository = published_repository("push-receipt");
    let mut terminal = terminal_for(&repository);
    open_dialog(&mut terminal, REMOTE_PUSH_TOOL_LABEL);
    terminal.send_keys("enter");
    assert!(
        terminal.driver().panel_error().is_none(),
        "the push reaches the backend, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.driver().status_message(),
        Some((PUSH_RECEIPT, StatusTone::Info)),
        "the push reports its receipt, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(
        terminal.screen_text().contains(PUSH_RECEIPT),
        "the receipt is on the status bar"
    );
    for _ in 0..STATUS_MESSAGE_LIFETIME_TICKS - 1 {
        terminal.tick();
    }
    assert_eq!(
        terminal.driver().status_message(),
        Some((PUSH_RECEIPT, StatusTone::Info)),
        "the receipt stays visible for three seconds"
    );
    terminal.screenshot("remote_dialogs/push-receipt");
}

#[test]
fn a_successful_fetch_reports_a_receipt() {
    let repository = published_repository("fetch-receipt");
    let remote_side = remote_side_copy("fetch-receipt", &repository);
    advance_the_remote(&remote_side, "remote moves ahead");
    let mut terminal = terminal_for(&repository);
    open_dialog(&mut terminal, REMOTE_FETCH_TOOL_LABEL);
    terminal.send_keys("enter");
    assert!(
        terminal.driver().panel_error().is_none(),
        "the fetch reaches the backend, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.driver().status_message(),
        Some((FETCH_RECEIPT, StatusTone::Info)),
        "the fetch reports its receipt, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(
        terminal.screen_text().contains(FETCH_RECEIPT),
        "the receipt is on the status bar"
    );
    for _ in 0..STATUS_MESSAGE_LIFETIME_TICKS - 1 {
        terminal.tick();
    }
    assert_eq!(
        terminal.driver().status_message(),
        Some((FETCH_RECEIPT, StatusTone::Info)),
        "the receipt stays visible for three seconds"
    );
    terminal.screenshot("remote_dialogs/fetch-receipt");
}
