use ratatui::layout::Rect;

use gitdeck_app::App;
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    CONTINUATION_MARKER_ASCII, CONTINUATION_MARKER_UNICODE, DIFF_MARKER_COLUMN,
    DIFF_NEW_NUMBER_COLUMN, DIFF_OLD_NUMBER_COLUMN, DIFF_TEXT_COLUMN,
};

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const LONG_LINE_PATH: &str = "src/long_line.txt";
const LONG_LINE_LENGTH: usize = 400;
const CHANGES_ROW_SCAN_LIMIT: usize = 32;
const HORIZONTAL_STEP_COLUMNS: u16 = 8;
const SOFT_WRAP_ON_STATUS: &str = "Soft wrap: on";
const SOFT_WRAP_OFF_STATUS: &str = "Soft wrap: off";
const CLIPPED_LINE_MARKER: &str = ">";
const REMOVED_WORD_HIGHLIGHT: Color = Color::Rgb(0x6B, 0x3A, 0x3A);
const ADDED_WORD_HIGHLIGHT: Color = Color::Rgb(0x3A, 0x6B, 0x47);
const REMOVED_LINE_BACKGROUND: Color = Color::Rgb(0x3A, 0x2A, 0x2A);
const ADDED_LINE_BACKGROUND: Color = Color::Rgb(0x25, 0x40, 0x2F);

fn diff_terminal(width: u16, height: u16, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::DiffShowcase,
        width,
        height,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the diff showcase repository opens")
        },
    );
    terminal.resize(width, height);
    terminal
}

fn select_file(terminal: &mut TestTerminal<App>, path: &str) {
    terminal.send_keys("ctrl+k");
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

fn diff_area(terminal: &TestTerminal<App>) -> Rect {
    let layout = terminal.driver().layout();
    layout.rows_above_bottom_rule(layout.log_content_rect())
}

fn diff_text_width(terminal: &TestTerminal<App>) -> u16 {
    diff_area(terminal).width.saturating_sub(DIFF_TEXT_COLUMN)
}

fn display_row_count(text_width: u16) -> usize {
    let width = usize::from(text_width);
    LONG_LINE_LENGTH.div_ceil(width)
}

fn diff_row_count(text_width: u16) -> usize {
    display_row_count(text_width) * 2 + 1
}

fn symbol_at(terminal: &TestTerminal<App>, column: u16, row: u16) -> String {
    terminal.cell(column, row).symbol.clone()
}

fn diff_row_symbol(terminal: &TestTerminal<App>, column_offset: u16, row: u16) -> String {
    symbol_at(terminal, diff_area(terminal).x + column_offset, row)
}

fn continuation_marker_of(caps: CapsProfile) -> String {
    match caps {
        CapsProfile::Rich | CapsProfile::Standard => CONTINUATION_MARKER_UNICODE.to_string(),
        CapsProfile::Basic => CONTINUATION_MARKER_ASCII.to_string(),
    }
}

fn status_text(terminal: &TestTerminal<App>) -> Option<String> {
    terminal
        .driver()
        .status_message()
        .map(|(text, _)| text.to_string())
}

#[test]
fn a_long_line_wraps_into_continuation_rows() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        open_selected_diff(&mut terminal, LONG_LINE_PATH);
        let text_width = diff_text_width(&terminal);
        let removed_row_count = display_row_count(text_width);
        assert!(
            removed_row_count > 1,
            "{caps:?} the 400 character line needs more than one row at {text_width} columns"
        );
        let top = diff_area(&terminal).y;
        assert_eq!(
            diff_row_symbol(&terminal, DIFF_MARKER_COLUMN, top + 1),
            "-",
            "{caps:?} the first display row keeps the removed marker"
        );
        let expected_marker = continuation_marker_of(caps);
        for offset in 1..removed_row_count {
            let row = top + 1 + u16::try_from(offset).expect("the row fits a u16");
            assert_eq!(
                diff_row_symbol(&terminal, DIFF_MARKER_COLUMN, row),
                expected_marker,
                "{caps:?} row {row} carries the continuation marker"
            );
            assert_eq!(
                diff_row_symbol(&terminal, DIFF_OLD_NUMBER_COLUMN, row).trim(),
                "",
                "{caps:?} row {row} leaves the old line number empty"
            );
            assert_eq!(
                diff_row_symbol(&terminal, DIFF_NEW_NUMBER_COLUMN, row).trim(),
                "",
                "{caps:?} row {row} leaves the new line number empty"
            );
        }
        let added_top = top + 1 + u16::try_from(removed_row_count).expect("the row fits a u16");
        assert_eq!(
            diff_row_symbol(&terminal, DIFF_MARKER_COLUMN, added_top),
            "+",
            "{caps:?} the first added display row keeps the added marker"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("soft_wrap/long-line-wrapped");
        }
    }
}

#[test]
fn continuation_rows_keep_the_change_background_and_highlight() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, LONG_LINE_PATH);
    let text_width = diff_text_width(&terminal);
    let removed_row_count = display_row_count(text_width);
    let top = diff_area(&terminal).y;
    let last_removed_row = top + u16::try_from(removed_row_count).expect("the row fits a u16");
    let added_top = last_removed_row + 1;
    let last_added_row = added_top + u16::try_from(removed_row_count - 1).expect("the row fits");
    assert_ne!(
        last_removed_row, added_top,
        "the wrap produces at least one continuation row"
    );
    for (row, line_background, highlight_background) in [
        (
            last_removed_row,
            REMOVED_LINE_BACKGROUND,
            REMOVED_WORD_HIGHLIGHT,
        ),
        (last_added_row, ADDED_LINE_BACKGROUND, ADDED_WORD_HIGHLIGHT),
    ] {
        let text_cell = terminal.cell(diff_area(&terminal).x + DIFF_TEXT_COLUMN, row);
        assert_eq!(
            text_cell.background, highlight_background,
            "row {row} keeps the word highlight on its continuation text"
        );
        let marker_cell = terminal.cell(diff_area(&terminal).x + DIFF_MARKER_COLUMN, row);
        assert_eq!(
            marker_cell.background, line_background,
            "row {row} keeps the change background under the continuation marker"
        );
        assert_eq!(
            marker_cell.symbol,
            CONTINUATION_MARKER_UNICODE.to_string(),
            "row {row} marks the wrapped line as a continuation"
        );
    }
    terminal.screenshot("soft_wrap/continuation-row-highlight");
    let mut unwrapped = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut unwrapped, LONG_LINE_PATH);
    unwrapped.send_keys("alt+z");
    assert_eq!(
        unwrapped
            .cell(diff_area(&unwrapped).x + DIFF_TEXT_COLUMN, top + 1)
            .background,
        REMOVED_WORD_HIGHLIGHT,
        "the unwrapped first row keeps the same highlight"
    );
}

#[test]
fn space_on_a_continuation_row_toggles_its_hunk() {
    for caps in [CapsProfile::Rich, CapsProfile::Basic] {
        let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        open_selected_diff(&mut terminal, LONG_LINE_PATH);
        terminal.send_keys("alt+9");
        assert_eq!(terminal.focused_pane(), PaneId::Log);
        terminal.send_keys("j");
        terminal.send_keys("j");
        assert_eq!(terminal.driver().diff_scroll(), 2);
        let top = diff_area(&terminal).y;
        assert_eq!(
            diff_row_symbol(&terminal, DIFF_MARKER_COLUMN, top),
            continuation_marker_of(caps),
            "{caps:?} the first visible row is a continuation row"
        );
        terminal.send_keys("space");
        assert_eq!(
            terminal.driver().checked_hunks_for_path(LONG_LINE_PATH),
            vec![0],
            "{caps:?} space on a continuation row checks the hunk of its logical line"
        );
        terminal.send_keys("space");
        assert!(
            terminal
                .driver()
                .checked_hunks_for_path(LONG_LINE_PATH)
                .is_empty(),
            "{caps:?} a second space unchecks the same hunk"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("soft_wrap/continuation-row-hunk-check");
        }
    }
}

#[test]
fn the_wrap_toggle_restores_horizontal_scrolling_when_off() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, LONG_LINE_PATH);
    terminal.send_keys("alt+9");
    assert!(terminal.driver().soft_wrap());
    terminal.send_keys("right");
    assert_eq!(
        terminal.driver().diff_horizontal_offset(),
        0,
        "the right key leaves the wrapped diff anchored"
    );
    terminal.send_keys("alt+z");
    assert!(!terminal.driver().soft_wrap());
    assert_eq!(
        status_text(&terminal).as_deref(),
        Some(SOFT_WRAP_OFF_STATUS)
    );
    terminal.send_keys("right");
    assert_eq!(
        terminal.driver().diff_horizontal_offset(),
        HORIZONTAL_STEP_COLUMNS,
        "the right key scrolls the unwrapped diff"
    );
    let area = diff_area(&terminal);
    assert_eq!(
        symbol_at(&terminal, area.right() - 1, area.y + 1),
        CLIPPED_LINE_MARKER,
        "the unwrapped long line marks its truncated last column"
    );
    terminal.screenshot("soft_wrap/horizontal-scroll-when-off");
    terminal.send_keys("alt+z");
    assert!(terminal.driver().soft_wrap());
    assert_eq!(status_text(&terminal).as_deref(), Some(SOFT_WRAP_ON_STATUS));
    assert_eq!(
        terminal.driver().diff_horizontal_offset(),
        0,
        "turning the wrap back on anchors the diff again"
    );
}

#[test]
fn scrolling_steps_by_display_rows_when_wrapped() {
    let mut terminal = diff_terminal(NARROW_WIDTH, NARROW_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, LONG_LINE_PATH);
    terminal.send_keys("alt+9");
    let text_width = diff_text_width(&terminal);
    let expected_rows = diff_row_count(text_width);
    assert!(
        expected_rows > 3,
        "the narrow pane wraps the long line into {expected_rows} display rows"
    );
    let top_left = symbol_at(&terminal, diff_area(&terminal).x, diff_area(&terminal).y);
    terminal.send_keys("j");
    assert_eq!(
        terminal.driver().diff_scroll(),
        1,
        "one press moves one display row"
    );
    assert_ne!(
        symbol_at(&terminal, diff_area(&terminal).x, diff_area(&terminal).y),
        top_left,
        "the top row changed after a single display row step"
    );
    for _ in 0..expected_rows * 2 {
        terminal.send_keys("j");
    }
    assert_eq!(
        terminal.driver().diff_scroll(),
        expected_rows - 1,
        "scrolling stops at the last display row, not the last logical row"
    );
    let page_rows = usize::from(terminal.driver().layout().commits.height.max(1));
    terminal.send_keys("pageup");
    assert_eq!(
        terminal.driver().diff_scroll(),
        (expected_rows - 1).saturating_sub(page_rows),
        "page up steps a page of display rows"
    );
    terminal.screenshot("soft_wrap/scrolling-narrow-wrapped");
}

#[test]
fn basic_caps_uses_an_ascii_continuation_marker() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Basic);
    open_selected_diff(&mut terminal, LONG_LINE_PATH);
    let text_width = diff_text_width(&terminal);
    assert!(display_row_count(text_width) > 2);
    let top = diff_area(&terminal).y;
    let marker = diff_row_symbol(&terminal, DIFF_MARKER_COLUMN, top + 2);
    assert_eq!(
        marker,
        CONTINUATION_MARKER_ASCII.to_string(),
        "the basic profile falls back to the ascii continuation marker"
    );
    assert!(marker.is_ascii(), "the basic marker stays ascii");
    assert!(!CONTINUATION_MARKER_UNICODE.is_ascii());
    terminal.screenshot("soft_wrap/basic-continuation-marker");
}
