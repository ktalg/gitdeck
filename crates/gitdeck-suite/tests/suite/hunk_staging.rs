use std::path::PathBuf;

use gitdeck_app::App;
use gitdeck_core::PaneId;
use gitdeck_testkit::{fixture_repo, prelude::*};
use gitdeck_tui::{DIFF_HUNK_TEXT_COLUMN, LOG_TAB_TITLE};

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const CHANGES_ROW_SCAN_LIMIT: usize = 32;
const TWO_HUNK_FILE_PATH: &str = "src/large.txt";
const OTHER_FILE_PATH: &str = "src/modified.rs";
const MODIFIED_APP_PATH: &str = "src/app.rs";
const UNCHECKED_HUNK_LABEL: &str = "[ ]";
const CHECKED_HUNK_LABEL: &str = "[x]";
const COMMIT_DIFF_HUNK_STAGING_STATUS: &str = "hunk staging works on working tree changes only";
const ACCENT_COLOR: Color = Color::Rgb(0x35, 0x74, 0xF0);

fn showcase_terminal(caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::DiffShowcase,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the diff showcase repository opens")
        },
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
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

fn open_selected_diff(terminal: &mut TestTerminal<App>, path: &str) {
    select_file(terminal, path);
    terminal.send_keys("enter");
}

fn diff_pane_left(terminal: &TestTerminal<App>) -> u16 {
    terminal.driver().layout().log_area.x
}

fn first_diff_row(terminal: &TestTerminal<App>) -> u16 {
    terminal
        .find_line("@@")
        .expect("the hunk header row is visible")
}

fn hunk_header_label(terminal: &TestTerminal<App>, row: u16) -> String {
    let left = diff_pane_left(terminal);
    (0..3)
        .map(|offset| terminal.cell(left + offset, row).symbol.clone())
        .collect()
}

fn focus_the_diff_pane(terminal: &mut TestTerminal<App>) {
    terminal.send_keys("alt+9");
    terminal.assert_focused(PaneId::Log);
}

#[test]
fn hunk_rows_show_a_checkbox_marker() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = showcase_terminal(caps);
        open_selected_diff(&mut terminal, TWO_HUNK_FILE_PATH);
        focus_the_diff_pane(&mut terminal);
        let row = first_diff_row(&terminal);
        assert_eq!(
            hunk_header_label(&terminal, row),
            UNCHECKED_HUNK_LABEL,
            "{caps:?} the hunk header row keeps an unchecked marker"
        );
        assert_eq!(
            terminal
                .cell(diff_pane_left(&terminal) + DIFF_HUNK_TEXT_COLUMN, row)
                .symbol,
            "@",
            "{caps:?} the hunk header text follows the marker column"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("hunk_staging/unchecked-hunk");
        }
    }
}

#[test]
fn space_toggles_a_hunk_and_the_selection_survives_a_file_switch() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = showcase_terminal(caps);
        open_selected_diff(&mut terminal, TWO_HUNK_FILE_PATH);
        focus_the_diff_pane(&mut terminal);
        let row = first_diff_row(&terminal);
        terminal.send_keys("space");
        assert_eq!(
            hunk_header_label(&terminal, row),
            CHECKED_HUNK_LABEL,
            "{caps:?} the space key checks the hunk under the cursor row"
        );
        assert_eq!(
            terminal.cell(diff_pane_left(&terminal), row).foreground,
            ACCENT_COLOR,
            "{caps:?} the checked marker uses the accent color"
        );
        assert_eq!(
            terminal.driver().checked_hunks_for_path(TWO_HUNK_FILE_PATH),
            vec![0],
            "{caps:?} the checked hunk is recorded for its path"
        );
        open_selected_diff(&mut terminal, OTHER_FILE_PATH);
        assert!(terminal.screen_text().contains(LOG_TAB_TITLE));
        terminal.send_keys("ctrl+w");
        open_selected_diff(&mut terminal, TWO_HUNK_FILE_PATH);
        focus_the_diff_pane(&mut terminal);
        assert_eq!(
            hunk_header_label(&terminal, first_diff_row(&terminal)),
            CHECKED_HUNK_LABEL,
            "{caps:?} the checked hunk survives switching files"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("hunk_staging/checked-hunk");
        }
    }
}

#[test]
fn space_clears_the_hunk_that_is_checked_again() {
    let mut terminal = showcase_terminal(CapsProfile::Rich);
    open_selected_diff(&mut terminal, TWO_HUNK_FILE_PATH);
    focus_the_diff_pane(&mut terminal);
    let row = first_diff_row(&terminal);
    terminal.send_keys("space");
    terminal.send_keys("space");
    assert_eq!(hunk_header_label(&terminal, row), UNCHECKED_HUNK_LABEL);
    assert!(terminal
        .driver()
        .checked_hunks_for_path(TWO_HUNK_FILE_PATH)
        .is_empty());
}

#[test]
fn commit_diff_rejects_hunk_staging() {
    let mut terminal = showcase_terminal(CapsProfile::Rich);
    terminal.send_keys("alt+9");
    terminal.send_keys("enter");
    assert!(
        !terminal.driver().hunk_staging_available(),
        "a commit diff never offers hunk staging"
    );
    terminal.send_keys("space");
    let status = terminal
        .driver()
        .status_message()
        .map(|(text, _)| text.to_string());
    assert_eq!(status.as_deref(), Some(COMMIT_DIFF_HUNK_STAGING_STATUS));
    assert!(
        terminal
            .driver()
            .checked_hunks_for_path(TWO_HUNK_FILE_PATH)
            .is_empty()
            && terminal
                .driver()
                .checked_hunks_for_path(OTHER_FILE_PATH)
                .is_empty(),
        "the rejected hunk staging records nothing"
    );
    terminal.screenshot("hunk_staging/commit-diff-rejected");
}

#[test]
fn hunk_header_alignment_is_preserved_with_the_marker_column() {
    let mut terminal = showcase_terminal(CapsProfile::Rich);
    open_selected_diff(&mut terminal, TWO_HUNK_FILE_PATH);
    focus_the_diff_pane(&mut terminal);
    let row = first_diff_row(&terminal);
    let unchecked_header = terminal.screen_lines()[usize::from(row)].clone();
    terminal.send_keys("space");
    let checked_header = terminal.screen_lines()[usize::from(row)].clone();
    let expected_column = usize::from(diff_pane_left(&terminal) + DIFF_HUNK_TEXT_COLUMN);
    assert_eq!(
        first_column_of(&unchecked_header, "@@"),
        Some(expected_column),
        "the unchecked header follows the marker column"
    );
    assert_eq!(
        first_column_of(&checked_header, "@@"),
        Some(expected_column),
        "the checked header keeps the unchecked column"
    );
    assert_eq!(
        first_column_of(&unchecked_header, UNCHECKED_HUNK_LABEL),
        Some(usize::from(diff_pane_left(&terminal)))
    );
    assert_eq!(
        first_column_of(&checked_header, CHECKED_HUNK_LABEL),
        Some(usize::from(diff_pane_left(&terminal)))
    );
}

fn first_column_of(screen_line: &str, needle: &str) -> Option<usize> {
    let line: String = screen_line.chars().collect();
    line.find(needle)
        .map(|byte_index| line[..byte_index].chars().count())
}

fn copied_showcase_repository(name: &str) -> PathBuf {
    let target = repository_root()
        .join("artifacts/tmp/suite-hunk-staging")
        .join(name);
    if target.exists() {
        std::fs::remove_dir_all(&target).expect("the stale repository copy is removed");
    }
    copy_directory(&fixture_repo(FixtureRepo::DiffShowcase), &target)
        .expect("the showcase repository is copied");
    target
}

fn copied_repository_terminal(name: &str) -> TestTerminal<App> {
    let repository_path = copied_showcase_repository(name);
    TestTerminal::new(
        App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the copied repository opens"),
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    )
}

#[test]
fn the_hunk_staging_receipt_reports_the_staged_hunks() {
    let mut terminal = copied_repository_terminal("receipt");
    open_selected_diff(&mut terminal, TWO_HUNK_FILE_PATH);
    focus_the_diff_pane(&mut terminal);
    terminal.send_keys("space");
    assert_eq!(
        terminal.driver().checked_hunks_for_path(TWO_HUNK_FILE_PATH),
        vec![0]
    );
    terminal.driver_mut().set_commit_message("stage two hunks");
    terminal
        .driver_mut()
        .commit()
        .expect("the staged hunks commit");
    assert_eq!(
        terminal.driver().status_message().map(|(text, _)| text),
        Some("Staged 1 hunks in 1 files")
    );
    assert!(
        terminal
            .driver()
            .checked_hunks_for_path(TWO_HUNK_FILE_PATH)
            .is_empty(),
        "the commit clears the hunk selection"
    );
}

fn copied_dirty_repository(name: &str) -> PathBuf {
    let target = repository_root()
        .join("artifacts/tmp/suite-hunk-staging")
        .join(name);
    if target.exists() {
        std::fs::remove_dir_all(&target).expect("the stale repository copy is removed");
    }
    copy_directory(&fixture_repo(FixtureRepo::Dirty), &target)
        .expect("the dirty repository is copied");
    target
}

#[test]
fn a_hunk_commit_succeeds_on_a_file_with_staged_changes() {
    let repository_path = copied_dirty_repository("modified-staged-file");
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the copied repository opens"),
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    open_selected_diff(&mut terminal, MODIFIED_APP_PATH);
    focus_the_diff_pane(&mut terminal);
    terminal.send_keys("space");
    assert_eq!(
        terminal.driver().checked_hunks_for_path(MODIFIED_APP_PATH),
        vec![0],
        "the hunk of the staged and modified file is checked"
    );
    terminal
        .driver_mut()
        .set_commit_message("commit the working tree hunk");
    terminal
        .driver_mut()
        .commit()
        .expect("the hunk commit succeeds on a staged and modified file");

    assert_eq!(
        terminal.driver().panel_error(),
        None,
        "the commit of a staged and modified file reports no error"
    );
    assert!(
        !terminal
            .driver()
            .changes()
            .iter()
            .any(|change| change.path == MODIFIED_APP_PATH),
        "the committed file leaves the changes list"
    );
    assert!(
        std::fs::read_to_string(repository_path.join(MODIFIED_APP_PATH))
            .expect("the committed file stays readable")
            .contains("shutdown()"),
        "the work tree keeps its content"
    );
}
