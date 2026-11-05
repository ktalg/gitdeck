use gitdeck_app::{App, EditorTab};
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{DividerId, DIVIDER_WIDTH, FILE_NAME_COLUMN};

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const COMPACT_WIDTH: u16 = 80;
const COMPACT_HEIGHT: u16 = 24;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const LONG_LINE_PATH: &str = "src/long_line.txt";
const LONG_LINE_FILE_TITLE: &str = "long_line.txt (Working Tree)";
const CHANGES_ROW_SCAN_LIMIT: usize = 32;
const BODY_ROW: u16 = 10;
const DIVIDER_RESIZE_STEP: u16 = 6;
const DIVIDER_GLYPH: char = '│';

fn diff_layout_terminal(width: u16, height: u16, caps: CapsProfile) -> TestTerminal<App> {
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

fn open_long_line_diff(terminal: &mut TestTerminal<App>) {
    select_file(terminal, LONG_LINE_PATH);
    terminal.send_keys("enter");
    assert_eq!(
        terminal.driver().active_tab(),
        EditorTab::Diff(String::from(LONG_LINE_PATH))
    );
}

fn details_width(terminal: &TestTerminal<App>) -> u16 {
    terminal.driver().layout().details.width
}

fn commits_width(terminal: &TestTerminal<App>) -> u16 {
    terminal.driver().layout().commits.width
}

fn divider_glyph_count(terminal: &TestTerminal<App>, row: u16) -> usize {
    terminal
        .screen_lines()
        .get(usize::from(row))
        .map(|line| {
            line.chars()
                .filter(|symbol| *symbol == DIVIDER_GLYPH)
                .count()
        })
        .unwrap_or(0)
}

#[test]
fn a_diff_tab_gives_the_details_width_to_the_diff() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = diff_layout_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let details_before = details_width(&terminal);
        let commits_before = commits_width(&terminal);
        assert!(
            details_before > 0,
            "{caps:?} the reference width shows the details column"
        );
        open_long_line_diff(&mut terminal);
        let layout = terminal.driver().layout();
        assert_eq!(
            layout.details.width, 0,
            "{caps:?} the details column yields"
        );
        assert_eq!(
            terminal.pane_rect(PaneId::Details),
            None,
            "{caps:?} the yielded details column reports no pane"
        );
        assert_eq!(
            layout.commits.width,
            commits_before + details_before + DIVIDER_WIDTH,
            "{caps:?} the diff takes the whole details column and its divider"
        );
        assert_eq!(
            layout.commits.right(),
            layout.log_area.right(),
            "{caps:?} the diff reaches the right screen edge"
        );
        terminal.screenshot(&format!("diff_layout/give-{caps:?}"));
    }
}

#[test]
fn closing_the_diff_tab_restores_the_details_width() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = diff_layout_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let details_before = details_width(&terminal);
        let commits_before = commits_width(&terminal);
        open_long_line_diff(&mut terminal);
        assert_eq!(
            details_width(&terminal),
            0,
            "{caps:?} the diff hides details"
        );
        terminal.send_keys("ctrl+w");
        assert_eq!(
            terminal.driver().active_tab(),
            EditorTab::Log,
            "{caps:?} ctrl+w returns to the log tab"
        );
        assert_eq!(
            details_width(&terminal),
            details_before,
            "{caps:?} the details column returns at its previous width"
        );
        assert_eq!(
            commits_width(&terminal),
            commits_before,
            "{caps:?} the log gives the width back"
        );
        terminal.screenshot(&format!("diff_layout/restore-{caps:?}"));
    }
}

#[test]
fn a_dragged_details_width_survives_the_diff_tab() {
    let mut terminal = diff_layout_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    let dragged_details_width = details_width(&terminal) + DIVIDER_RESIZE_STEP;
    let divider_column = terminal.driver().layout().commits_details_divider.x;
    terminal.mouse_drag(
        (divider_column, BODY_ROW),
        (divider_column - DIVIDER_RESIZE_STEP, BODY_ROW),
    );
    assert_eq!(
        details_width(&terminal),
        dragged_details_width,
        "the drag widens the details column"
    );
    open_long_line_diff(&mut terminal);
    assert_eq!(
        details_width(&terminal),
        0,
        "the diff hides the dragged column"
    );
    terminal.send_keys("ctrl+w");
    assert_eq!(
        details_width(&terminal),
        dragged_details_width,
        "the dragged width comes back with the details column"
    );
    terminal.screenshot("diff_layout/dragged-width-restored");
}

#[test]
fn the_details_divider_disappears_while_a_diff_tab_shows() {
    let mut terminal = diff_layout_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    let divider_column = terminal.driver().layout().commits_details_divider.x;
    assert_eq!(
        divider_glyph_count(&terminal, BODY_ROW),
        2,
        "the log tab draws the changes and the details dividers"
    );
    assert_eq!(
        terminal.driver().hit_divider(divider_column, BODY_ROW),
        Some(DividerId::CommitsDetails),
        "the details divider answers the pointer while the log tab shows"
    );
    open_long_line_diff(&mut terminal);
    let layout = terminal.driver().layout();
    assert_eq!(
        layout.commits_details_divider.width, 0,
        "the yielded details column drops its divider"
    );
    assert_eq!(
        layout.changes_log_divider.width, DIVIDER_WIDTH,
        "the changes divider stays"
    );
    assert_eq!(
        divider_glyph_count(&terminal, BODY_ROW),
        1,
        "the diff screen draws only the changes divider"
    );
    assert_eq!(
        terminal.driver().hit_divider(divider_column, BODY_ROW),
        None,
        "the vanished divider answers no pointer"
    );
    terminal.screenshot("diff_layout/details-divider-gone");
}

#[test]
fn focus_moves_out_of_details_when_it_collapses() {
    let mut terminal = diff_layout_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    terminal.send_keys("alt+3");
    assert_eq!(
        terminal.focused_pane(),
        PaneId::Details,
        "alt+3 focuses the details column"
    );
    let changes = terminal
        .pane_rect(PaneId::Changes)
        .expect("the changes pane is visible");
    let row = terminal.find_line(LONG_LINE_FILE_TITLE).unwrap_or_else(|| {
        terminal
            .find_line("long_line.txt")
            .expect("the long line row")
    });
    terminal.mouse_double_click(changes.x + FILE_NAME_COLUMN, row);
    assert_eq!(
        terminal.driver().active_tab(),
        EditorTab::Diff(String::from(LONG_LINE_PATH)),
        "the double click opens the diff"
    );
    assert_eq!(
        terminal.focused_pane(),
        PaneId::Log,
        "the focus leaves the collapsed details column"
    );
    terminal.send_keys("alt+3");
    assert_eq!(
        terminal.focused_pane(),
        PaneId::Log,
        "the collapsed details column refuses focus"
    );
}

#[test]
fn a_compact_screen_keeps_its_single_divider_with_a_diff_tab() {
    let mut terminal = diff_layout_terminal(COMPACT_WIDTH, COMPACT_HEIGHT, CapsProfile::Rich);
    assert_eq!(details_width(&terminal), 0, "eighty columns hide details");
    let commits_before = commits_width(&terminal);
    open_long_line_diff(&mut terminal);
    assert_eq!(details_width(&terminal), 0, "the diff keeps details hidden");
    assert_eq!(
        commits_width(&terminal),
        commits_before,
        "the compact log column keeps its width"
    );
    assert_eq!(
        divider_glyph_count(&terminal, BODY_ROW),
        1,
        "the compact screen keeps its single divider"
    );
}
