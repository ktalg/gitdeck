use std::time::{Duration, Instant};

use gitdeck_app::{App, EditorTab};
use gitdeck_core::PaneId;
use gitdeck_git::DiffLineKind;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    BINARY_FILE_LABEL, BOTTOM_RULE_HEIGHT, DIFF_HUNK_TEXT_COLUMN, DIFF_MARKER_COLUMN,
    DIFF_NEW_NUMBER_COLUMN, DIFF_OLD_NUMBER_COLUMN, DIFF_TEXT_COLUMN, LOG_TAB_TITLE,
    WORKING_TREE_TAB_SUFFIX,
};

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const CHANGES_ROW_SCAN_LIMIT: usize = 32;
const PANEL_BACKGROUND: Color = Color::Rgb(0x2B, 0x2D, 0x30);
const EDITOR_BACKGROUND: Color = Color::Rgb(0x1E, 0x1F, 0x22);
const MODIFIED_FILE_PATH: &str = "src/modified.rs";
const MODIFIED_FILE_TITLE: &str = "modified.rs (Working Tree)";
const UNTRACKED_FILE_PATH: &str = "src/untracked_new.rs";
const UNTRACKED_FILE_TITLE: &str = "untracked_new.rs (Working Tree)";
const BINARY_FILE_PATH: &str = "assets/logo.bin";
const BINARY_FILE_TITLE: &str = "logo.bin (Working Tree)";
const MISSING_NEWLINE_FILE_PATH: &str = "src/no_newline.txt";
const LONG_LINE_FILE_PATH: &str = "src/long_line.txt";
const LARGE_FILE_PATH: &str = "src/large.txt";
const LARGE_FILE_HUNK_COUNT: usize = 10;
const LARGE_FILE_TIME_LIMIT: Duration = Duration::from_millis(300);
const UNTRACKED_LINE_COUNT: usize = 10;
const NO_NEWLINE_MARKER: &str = "\\ No newline at end of file";
const CJK_FILE_PATH: &str = "文档/说明.md";
const CJK_FILE_TITLE: &str = "说明.md (Working Tree)";
const CJK_MISSING_GLYPHS: [char; 10] = ['一', '二', '内', '容', '文', '档', '明', '第', '行', '说'];
const CELL_PIXEL_WIDTH: u32 = 16;
const CELL_PIXEL_HEIGHT: u32 = 32;
const SHOWCASE_MISSING_GLYPHS: [char; 4] = ['文', '档', '明', '说'];
const ASCII_DIFF_FILE_PATH: &str = "src/app.rs";
const DIFF_HORIZONTAL_STEP_COLUMNS: u16 = 8;
const RULE_GLYPHS: [&str; 4] = ["\u{2500}", "-", "\u{2534}", "+"];
const ADDED_WORD_HIGHLIGHT_BACKGROUND: Color = Color::Rgb(0x3A, 0x6B, 0x47);
const ADDED_LINE_BACKGROUND_COLOR: Color = Color::Rgb(0x25, 0x40, 0x2F);
const CHANGED_DIGIT_COLUMN: u16 = 26;
const WORD_HIGHLIGHTS_OFF_STATUS: &str = "Word highlights: off";
const WORD_HIGHLIGHTS_ON_STATUS: &str = "Word highlights: on";

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

fn ascii_diff_terminal(caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the dirty repository opens")
        },
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    terminal
}

fn focused_changes_terminal(width: u16, height: u16) -> TestTerminal<App> {
    let mut terminal = diff_terminal(width, height, CapsProfile::Rich);
    terminal.send_keys("ctrl+k");
    terminal
}

fn expand_unversioned_files(terminal: &mut TestTerminal<App>) {
    select_file(terminal, "assets/logo.bin");
    for _ in 0..CHANGES_ROW_SCAN_LIMIT {
        terminal.send_keys("j");
    }
    terminal.send_keys("right");
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

fn log_left(terminal: &TestTerminal<App>) -> u16 {
    terminal.driver().layout().log_area.x
}

fn log_content_top(terminal: &TestTerminal<App>) -> u16 {
    terminal
        .pane_rect(PaneId::Log)
        .expect("the log pane is visible")
        .y
}

fn diff_right(terminal: &TestTerminal<App>) -> u16 {
    terminal.driver().layout().log_content_rect().right() - 1
}

fn tab_titles(terminal: &TestTerminal<App>) -> Vec<String> {
    terminal.driver().tab_titles()
}

fn marker_at_line(terminal: &TestTerminal<App>, needle: &str) -> String {
    let row = terminal
        .find_line(needle)
        .unwrap_or_else(|| panic!("no screen line holds {needle:?}"));
    terminal
        .cell(log_left(terminal) + DIFF_MARKER_COLUMN, row)
        .symbol
}

#[test]
fn enter_opens_diff_tab_for_selected_file() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        open_selected_diff(&mut terminal, MODIFIED_FILE_PATH);
        assert_eq!(
            terminal.driver().active_tab(),
            EditorTab::Diff(String::from(MODIFIED_FILE_PATH)),
            "{caps:?} active tab"
        );
        assert!(
            tab_titles(&terminal).contains(&String::from(MODIFIED_FILE_TITLE)),
            "{caps:?} tab titles {:?}",
            tab_titles(&terminal)
        );
        assert_eq!(
            terminal
                .driver()
                .file_diff()
                .map(|file_diff| file_diff.path.as_str()),
            Some(MODIFIED_FILE_PATH)
        );
        assert_eq!(
            tab_titles(&terminal).first(),
            Some(&String::from(LOG_TAB_TITLE)),
            "{caps:?} keeps the log tab first"
        );
    }
}

#[test]
fn diff_tab_title_uses_basename_and_working_tree() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, CJK_FILE_PATH);
    let titles = tab_titles(&terminal);
    let diff_title = titles
        .iter()
        .find(|title| title.ends_with(WORKING_TREE_TAB_SUFFIX))
        .expect("an open working tree tab");
    assert_eq!(diff_title, CJK_FILE_TITLE);
    assert!(
        !diff_title.contains('/'),
        "tab title {diff_title:?} holds a directory separator"
    );
}

#[test]
fn ctrl_w_closes_diff_and_restores_log_tab() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, MODIFIED_FILE_PATH);
    terminal.send_keys("ctrl+w");
    assert_eq!(terminal.driver().active_tab(), EditorTab::Log);
    assert_eq!(
        terminal.driver().file_diff().map(|diff| diff.path.as_str()),
        None
    );
    assert_eq!(tab_titles(&terminal), vec![String::from(LOG_TAB_TITLE)]);
    assert!(
        terminal.find_line(LOG_TAB_TITLE).is_some(),
        "the log tab is still drawn"
    );
    assert_eq!(terminal.find_line(MODIFIED_FILE_TITLE), None);
    terminal.send_keys("ctrl+w");
    assert_eq!(
        terminal.driver().active_tab(),
        EditorTab::Log,
        "closing the log tab does nothing"
    );
}

#[test]
fn escape_also_closes_the_active_diff_tab() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, MODIFIED_FILE_PATH);
    terminal.send_keys("esc");
    assert_eq!(terminal.driver().active_tab(), EditorTab::Log);
    assert_eq!(tab_titles(&terminal), vec![String::from(LOG_TAB_TITLE)]);
}

#[test]
fn modified_file_renders_markers_in_fixed_column() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        open_selected_diff(&mut terminal, MODIFIED_FILE_PATH);
        assert_eq!(
            marker_at_line(&terminal, "STEP_05: u32 = 50;"),
            "+",
            "{caps:?} added marker column"
        );
        assert_eq!(
            marker_at_line(&terminal, "STEP_05: u32 = 5;"),
            "-",
            "{caps:?} removed marker column"
        );
        assert_eq!(
            marker_at_line(&terminal, "STEP_02: u32 = 2;"),
            " ",
            "{caps:?} context marker column"
        );
    }
}

#[test]
fn hunk_header_renders_after_the_marker_column() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, MODIFIED_FILE_PATH);
    let row = terminal.find_line("@@ -").expect("a hunk header row");
    assert_eq!(
        terminal.cell(log_left(&terminal), row).symbol,
        "[",
        "the working tree hunk header starts with its checkbox column"
    );
    assert_eq!(
        terminal
            .cell(log_left(&terminal) + DIFF_HUNK_TEXT_COLUMN, row)
            .symbol,
        "@",
        "the hunk header text follows the checkbox column"
    );
    let first_hunk = terminal
        .driver()
        .file_diff()
        .and_then(|diff| diff.hunks.first().cloned())
        .expect("the modified file has a hunk");
    let expected_header = format!(
        "@@ -{},{} +{},{} @@",
        first_hunk.old_start, first_hunk.old_lines, first_hunk.new_start, first_hunk.new_lines
    );
    let header = terminal.screen_lines()[usize::from(row)].clone();
    assert!(
        header.contains(&expected_header),
        "hunk header {header:?} does not hold {expected_header:?}"
    );
    assert_eq!(
        log_content_top(&terminal),
        row,
        "the first diff row is the hunk header"
    );
}

#[test]
fn hunk_header_starts_after_the_marker_column_and_ignores_horizontal_offset() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, LONG_LINE_FILE_PATH);
    terminal.send_keys("alt+z");
    let first_hunk = terminal
        .driver()
        .file_diff()
        .and_then(|diff| diff.hunks.first().cloned())
        .expect("the long line file has a hunk");
    let expected_header = format!(
        "@@ -{},{} +{},{} @@",
        first_hunk.old_start, first_hunk.old_lines, first_hunk.new_start, first_hunk.new_lines
    );
    let row = terminal
        .find_line("@")
        .expect("the hunk header row is visible");
    let left = log_left(&terminal);
    assert_eq!(terminal.cell(left, row).symbol, "[");
    let header_before = terminal.screen_lines()[usize::from(row)].clone();
    for _ in 0..3 {
        terminal.send_keys("right");
    }
    assert_eq!(
        terminal.driver().diff_horizontal_offset(),
        DIFF_HORIZONTAL_STEP_COLUMNS * 3
    );
    assert_eq!(
        terminal.cell(left, row).symbol,
        "[",
        "the hunk header keeps its checkbox column"
    );
    assert_eq!(
        terminal.cell(left + DIFF_HUNK_TEXT_COLUMN, row).symbol,
        "@",
        "the hunk header text keeps its column"
    );
    assert_eq!(
        terminal.screen_lines()[usize::from(row)].clone(),
        header_before,
        "the hunk header ignores the horizontal offset"
    );
    assert!(
        header_before.contains(&expected_header),
        "hunk header {header_before:?} does not hold {expected_header:?}"
    );
    terminal.screenshot("layout_polish/diff-hunk-header");
}

fn diff_pane_lines(terminal: &TestTerminal<App>) -> Vec<String> {
    let left = usize::from(log_left(terminal));
    let right = usize::from(diff_right(terminal)) + 1;
    terminal
        .screen_lines()
        .iter()
        .map(|line| line.chars().skip(left).take(right - left).collect())
        .collect()
}

#[test]
fn horizontal_scroll_stops_at_the_longest_line() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = ascii_diff_terminal(caps);
        open_selected_diff(&mut terminal, ASCII_DIFF_FILE_PATH);
        let before = diff_pane_lines(&terminal);
        for _ in 0..3 {
            terminal.send_keys("right");
        }
        assert_eq!(
            terminal.driver().diff_horizontal_offset(),
            0,
            "{caps:?} a diff whose lines fit the pane keeps the offset at zero"
        );
        assert_eq!(
            diff_pane_lines(&terminal),
            before,
            "{caps:?} the right key leaves the fitting diff untouched"
        );
        assert!(
            !diff_pane_lines(&terminal)
                .iter()
                .any(|line| line.trim_end().ends_with('>')),
            "{caps:?} the fitting diff never shows the clipping marker"
        );
    }
}

#[test]
fn horizontal_scroll_clamps_to_the_longest_line() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        open_selected_diff(&mut terminal, LONG_LINE_FILE_PATH);
        terminal.send_keys("alt+z");
        let longest_line_width = terminal
            .driver()
            .file_diff()
            .map(|diff| {
                diff.hunks
                    .iter()
                    .flat_map(|hunk| hunk.lines.iter())
                    .map(|line| line.text.chars().count())
                    .max()
                    .unwrap_or(0)
            })
            .expect("the long line diff is open");
        let visible_text_width = u16::try_from(diff_pane_lines(&terminal)[0].chars().count())
            .expect("the pane width fits a u16")
            .saturating_sub(DIFF_TEXT_COLUMN);
        let expected_limit = u16::try_from(longest_line_width)
            .expect("the line width fits a u16")
            .saturating_sub(visible_text_width);
        assert!(
            expected_limit > DIFF_HORIZONTAL_STEP_COLUMNS,
            "{caps:?} the long line needs more than one step of scrolling"
        );
        for _ in 0..60 {
            terminal.send_keys("right");
        }
        assert_eq!(
            terminal.driver().diff_horizontal_offset(),
            expected_limit,
            "{caps:?} the offset saturates at the longest line width"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("regressions/diff-hscroll-clamped");
        }
    }
}

#[test]
fn bottom_separator_spans_the_full_width() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let log_area = terminal
            .pane_rect(PaneId::Log)
            .expect("the log pane is visible");
        let rule_row = log_area.bottom() - BOTTOM_RULE_HEIGHT;
        assert_rule_row_spans_the_width(&terminal, rule_row, caps, "the commit list");
        open_selected_diff(&mut terminal, MODIFIED_FILE_PATH);
        assert_rule_row_spans_the_width(&terminal, rule_row, caps, "the diff pane");
        terminal.screenshot(&format!("layout_polish/bottom-separator-{}", caps.as_str()));
    }
}

#[track_caller]
fn assert_rule_row_spans_the_width(
    terminal: &TestTerminal<App>,
    rule_row: u16,
    caps: CapsProfile,
    surface: &str,
) {
    for column in 0..REFERENCE_WIDTH {
        let symbol = terminal.cell(column, rule_row).symbol;
        assert!(
            RULE_GLYPHS.contains(&symbol.as_str()),
            "{caps:?} column {column} of {surface} holds {symbol:?} on the separator row"
        );
    }
}

#[test]
fn line_numbers_align_in_fixed_columns() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, MODIFIED_FILE_PATH);
    let left = log_left(&terminal);
    let row = terminal
        .find_line("STEP_04: u32 = 4;")
        .expect("the context row before the first change");
    for (column, expected) in [
        (DIFF_OLD_NUMBER_COLUMN, " "),
        (DIFF_OLD_NUMBER_COLUMN + 1, " "),
        (DIFF_OLD_NUMBER_COLUMN + 2, " "),
        (DIFF_OLD_NUMBER_COLUMN + 3, "4"),
    ] {
        assert_eq!(
            terminal.cell(left + column, row).symbol,
            expected,
            "old line number column {column}"
        );
    }
    assert_eq!(
        terminal.cell(left + DIFF_NEW_NUMBER_COLUMN + 3, row).symbol,
        "4"
    );
    assert_eq!(terminal.cell(left + DIFF_TEXT_COLUMN, row).symbol, "p");
    let two_digit_row = terminal
        .find_line("STEP_11: u32 = 11;")
        .expect("the context row before the second change");
    assert_eq!(
        terminal
            .cell(left + DIFF_OLD_NUMBER_COLUMN + 2, two_digit_row)
            .symbol,
        "1"
    );
    assert_eq!(
        terminal
            .cell(left + DIFF_OLD_NUMBER_COLUMN + 3, two_digit_row)
            .symbol,
        "1"
    );
}

#[test]
fn untracked_file_is_all_added_lines() {
    let mut terminal = focused_changes_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    expand_unversioned_files(&mut terminal);
    select_file(&mut terminal, UNTRACKED_FILE_PATH);
    terminal.send_keys("enter");
    assert_eq!(
        terminal.driver().active_tab(),
        EditorTab::Diff(String::from(UNTRACKED_FILE_PATH))
    );
    assert!(tab_titles(&terminal).contains(&String::from(UNTRACKED_FILE_TITLE)));
    let left = log_left(&terminal);
    let mut added_rows = 0;
    for row in log_content_top(&terminal)..REFERENCE_HEIGHT - 1 {
        let line = terminal.screen_lines()[usize::from(row)].clone();
        if !line.contains("pub fn extra_") {
            continue;
        }
        added_rows += 1;
        assert_eq!(
            terminal.cell(left + DIFF_MARKER_COLUMN, row).symbol,
            "+",
            "row {row} of the untracked diff"
        );
    }
    assert_eq!(added_rows, UNTRACKED_LINE_COUNT);
    assert_eq!(
        terminal.find_line("-pub fn extra").map(|_| ()),
        None,
        "an untracked file holds no removed lines"
    );
}

#[test]
fn binary_file_shows_binary_notice() {
    let mut terminal = focused_changes_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    select_file(&mut terminal, BINARY_FILE_PATH);
    terminal.send_keys("enter");
    assert_eq!(
        terminal.driver().active_tab(),
        EditorTab::Diff(String::from(BINARY_FILE_PATH))
    );
    assert_eq!(
        terminal.driver().file_diff().map(|diff| diff.binary),
        Some(true)
    );
    let row = terminal
        .find_line(BINARY_FILE_LABEL)
        .expect("the binary notice row");
    assert_eq!(
        terminal
            .cell(log_left(&terminal) + DIFF_TEXT_COLUMN, row)
            .symbol,
        "B",
        "the binary notice starts at the text column"
    );
    assert!(tab_titles(&terminal).contains(&String::from(BINARY_FILE_TITLE)));
}

#[test]
fn missing_trailing_newline_marker_is_rendered() {
    let mut terminal = focused_changes_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    select_file(&mut terminal, MISSING_NEWLINE_FILE_PATH);
    terminal.send_keys("enter");
    assert_eq!(
        terminal
            .driver()
            .file_diff()
            .map(|diff| diff.missing_newline_at_end),
        Some(true)
    );
    let row = terminal
        .find_line(NO_NEWLINE_MARKER)
        .expect("the missing newline marker row");
    assert_eq!(
        terminal
            .cell(log_left(&terminal) + DIFF_TEXT_COLUMN, row)
            .symbol,
        "\\",
        "the marker starts at the text column"
    );
}

#[test]
fn long_line_is_truncated_with_marker_at_last_column() {
    let mut terminal = focused_changes_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    select_file(&mut terminal, LONG_LINE_FILE_PATH);
    terminal.send_keys("enter");
    terminal.send_keys("alt+z");
    let right = diff_right(&terminal);
    let added_row = terminal.find_line("bbbb").expect("the long added line");
    let removed_row = terminal.find_line("aaaa").expect("the long removed line");
    for row in [added_row, removed_row] {
        assert_eq!(
            terminal.cell(right, row).symbol,
            ">",
            "the truncated line marks its last column"
        );
    }
    assert!(
        terminal.screen_lines()[usize::from(added_row)]
            .chars()
            .count()
            <= usize::from(right) + 1,
        "the long line overflows the screen"
    );
    assert_eq!(
        terminal.find_line(&"b".repeat(400)).map(|_| ()),
        None,
        "the whole 400 character line is never drawn"
    );
}

#[test]
fn cjk_diff_content_missing_glyphs_are_exact() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, CJK_FILE_PATH);
    terminal.assert_missing_glyphs(&CJK_MISSING_GLYPHS);
    for caps in EVERY_CAPS_PROFILE {
        let mut ascii_terminal = ascii_diff_terminal(caps);
        open_selected_diff(&mut ascii_terminal, ASCII_DIFF_FILE_PATH);
        ascii_terminal.assert_no_missing_glyphs();
    }
}

#[test]
fn changes_pane_uses_panel_background() {
    let terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    let changes = terminal
        .pane_rect(PaneId::Changes)
        .expect("the changes pane");
    assert_eq!(
        terminal.cell(changes.x, changes.y).background,
        PANEL_BACKGROUND,
        "the changes pane keeps the panel background"
    );
    let log = terminal.pane_rect(PaneId::Log).expect("the log pane");
    assert_eq!(
        terminal.cell(log.x, log.bottom() - 1).background,
        EDITOR_BACKGROUND,
        "the editor column uses the editor background"
    );
    assert_eq!(
        terminal.cell(0, 0).background,
        PANEL_BACKGROUND,
        "the top bar keeps the panel background"
    );
    assert_eq!(
        terminal.cell(changes.right(), changes.y).background,
        EDITOR_BACKGROUND,
        "the changes divider column follows the editor background"
    );
}

#[test]
fn diff_scrolls_with_j_k_and_pgup_pgdn() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, LARGE_FILE_PATH);
    terminal.send_keys("alt+9");
    terminal.assert_focused(PaneId::Log);
    let top_row = log_content_top(&terminal);
    let first_line = terminal.screen_lines()[usize::from(top_row)].clone();
    terminal.send_keys("j");
    assert_eq!(terminal.driver().diff_scroll(), 1);
    assert_ne!(
        terminal.screen_lines()[usize::from(top_row)].clone(),
        first_line
    );
    terminal.send_keys("k");
    assert_eq!(terminal.driver().diff_scroll(), 0);
    assert_eq!(
        terminal.screen_lines()[usize::from(top_row)].clone(),
        first_line
    );
    terminal.send_keys("k");
    assert_eq!(
        terminal.driver().diff_scroll(),
        0,
        "scrolling above the first row stops"
    );
    terminal.send_keys("pagedown");
    let page_scroll = terminal.driver().diff_scroll();
    assert!(page_scroll > 1, "page down scrolls a whole page");
    terminal.send_keys("pageup");
    assert_eq!(terminal.driver().diff_scroll(), 0);
    for _ in 0..200 {
        terminal.send_keys("pagedown");
    }
    let row_count = terminal
        .driver()
        .file_diff()
        .map(|diff| {
            diff.hunks
                .iter()
                .map(|hunk| hunk.lines.len() + 1)
                .sum::<usize>()
        })
        .expect("the large diff is open");
    assert_eq!(
        terminal.driver().diff_scroll(),
        row_count - 1,
        "scrolling stops at the last row"
    );
}

#[test]
fn large_diff_parses_within_budget() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    select_file(&mut terminal, LARGE_FILE_PATH);
    let started_at = Instant::now();
    terminal.send_keys("enter");
    let elapsed = started_at.elapsed();
    println!("opening the {LARGE_FILE_PATH} diff took {elapsed:?}");
    let file_diff = terminal
        .driver()
        .file_diff()
        .expect("the large diff is open");
    assert_eq!(file_diff.hunks.len(), LARGE_FILE_HUNK_COUNT);
    let added_line_count = file_diff
        .hunks
        .iter()
        .flat_map(|hunk| hunk.lines.iter())
        .filter(|line| line.kind == DiffLineKind::Added)
        .count();
    let removed_line_count = file_diff
        .hunks
        .iter()
        .flat_map(|hunk| hunk.lines.iter())
        .filter(|line| line.kind == DiffLineKind::Removed)
        .count();
    assert_eq!(added_line_count, LARGE_FILE_HUNK_COUNT);
    assert_eq!(removed_line_count, LARGE_FILE_HUNK_COUNT);
    assert!(
        elapsed < LARGE_FILE_TIME_LIMIT,
        "opening the large diff took {elapsed:?}, limit is {LARGE_FILE_TIME_LIMIT:?}"
    );
    assert_eq!(terminal.driver().panel_error(), None);
}

#[test]
fn reopening_an_open_diff_keeps_a_single_tab() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, MODIFIED_FILE_PATH);
    terminal.send_keys("ctrl+k");
    terminal.send_keys("enter");
    let titles = tab_titles(&terminal);
    assert_eq!(
        titles
            .iter()
            .filter(|title| *title == MODIFIED_FILE_TITLE)
            .count(),
        1
    );
    assert_eq!(titles.len(), 2);
    assert_eq!(
        terminal.driver().active_tab(),
        EditorTab::Diff(String::from(MODIFIED_FILE_PATH))
    );
}

#[test]
fn diff_screenshots_render_at_the_reference_sizes() {
    let scenarios: [(u16, u16, CapsProfile, &str, &str); 6] = [
        (
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            CapsProfile::Rich,
            MODIFIED_FILE_PATH,
            "diff/modified-160x48",
        ),
        (
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            CapsProfile::Rich,
            UNTRACKED_FILE_PATH,
            "diff/untracked-160x48",
        ),
        (
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            CapsProfile::Rich,
            BINARY_FILE_PATH,
            "diff/binary-160x48",
        ),
        (
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            CapsProfile::Basic,
            MODIFIED_FILE_PATH,
            "diff/basic-160x48",
        ),
        (
            NARROW_WIDTH,
            NARROW_HEIGHT,
            CapsProfile::Rich,
            MODIFIED_FILE_PATH,
            "diff/80x24",
        ),
        (
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            CapsProfile::Rich,
            LONG_LINE_FILE_PATH,
            "diff/long-line",
        ),
    ];
    for (width, height, caps, path, name) in scenarios {
        let mut terminal = diff_terminal(width, height, caps);
        if path == UNTRACKED_FILE_PATH {
            expand_unversioned_files(&mut terminal);
        }
        select_file(&mut terminal, path);
        terminal.send_keys("enter");
        let report = terminal.screenshot(name);
        assert_eq!(
            png_dimensions(&report.png_path),
            (
                u32::from(width) * CELL_PIXEL_WIDTH,
                u32::from(height) * CELL_PIXEL_HEIGHT
            ),
            "{name} png dimensions"
        );
        assert_eq!(
            sorted_glyphs(&report.missing_glyphs),
            sorted_glyphs(&SHOWCASE_MISSING_GLYPHS),
            "{name} missing glyphs"
        );
    }
}

fn sorted_glyphs(glyphs: &[char]) -> Vec<char> {
    let mut sorted_glyphs = glyphs.to_vec();
    sorted_glyphs.sort_unstable();
    sorted_glyphs
}

fn png_dimensions(png_path: &std::path::Path) -> (u32, u32) {
    let header = std::fs::read(png_path).expect("the screenshot png is readable");
    (
        u32::from_be_bytes([header[16], header[17], header[18], header[19]]),
        u32::from_be_bytes([header[20], header[21], header[22], header[23]]),
    )
}
#[test]
fn alt_w_toggles_word_highlights_and_reports_the_state() {
    let mut terminal = diff_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    open_selected_diff(&mut terminal, MODIFIED_FILE_PATH);
    let added_row = terminal
        .find_line("STEP_05: u32 = 50;")
        .expect("the added line is on screen");
    let changed_column = log_left(&terminal) + DIFF_TEXT_COLUMN + CHANGED_DIGIT_COLUMN;
    let shared_column = changed_column - 1;
    assert_eq!(
        terminal.cell(changed_column, added_row).background,
        ADDED_WORD_HIGHLIGHT_BACKGROUND,
        "the appended digit carries the word highlight background"
    );
    assert_eq!(
        terminal.cell(shared_column, added_row).background,
        ADDED_LINE_BACKGROUND_COLOR,
        "the shared digit keeps the line background"
    );
    let report = terminal.screenshot("diff/word-highlight-160x48");
    assert_eq!(
        png_dimensions(&report.png_path),
        (
            u32::from(REFERENCE_WIDTH) * CELL_PIXEL_WIDTH,
            u32::from(REFERENCE_HEIGHT) * CELL_PIXEL_HEIGHT
        ),
        "the word highlight screenshot covers every cell"
    );
    terminal.send_keys("alt+w");
    assert!(
        terminal.screen_text().contains(WORD_HIGHLIGHTS_OFF_STATUS),
        "turning word highlights off is reported, screen text was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.cell(changed_column, added_row).background,
        ADDED_LINE_BACKGROUND_COLOR,
        "the added row falls back to its line background"
    );
    terminal.send_keys("alt+w");
    assert!(
        terminal.screen_text().contains(WORD_HIGHLIGHTS_ON_STATUS),
        "turning word highlights on is reported, screen text was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.cell(changed_column, added_row).background,
        ADDED_WORD_HIGHLIGHT_BACKGROUND,
        "the added row regains its word highlight"
    );
}
