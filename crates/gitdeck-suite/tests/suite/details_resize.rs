use gitdeck_app::App;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{details_max_width, text_display_width, DividerId, BOTTOM_RULE_HEIGHT};

const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const WIDE_WIDTH: u16 = 220;
const WIDE_HEIGHT: u16 = 70;
const COMPACT_WIDTH: u16 = 80;
const COMPACT_HEIGHT: u16 = 24;
const BODY_ROW: u16 = 10;
const DETAILS_MIN_WIDTH: u16 = 24;
const OLD_DETAILS_FLOOR: u16 = 40;
const DIVIDER_GLYPH: char = '│';
const DETAILS_SEPARATOR_GLYPH: char = '─';
const DIRTY_SUBJECT: &str = "call run twice";
const CONTAINING_BRANCH_LABEL: &str = "In 1 branch: main";
const WIDE_LAYOUT_DETAILS_WIDTH: u16 = 61;
const DRAGGED_WIDE_DETAILS_WIDTH: u16 = 90;

fn details_terminal(width: u16, height: u16) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::Dirty,
        width,
        height,
        CapsProfile::Rich,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, CapsProfile::Rich, ThemeId::JbDark)
                .expect("fixture repository opens")
        },
    );
    terminal.resize(width, height);
    terminal
}

fn details_pane(terminal: &TestTerminal<App>) -> Rect {
    terminal
        .pane_rect(PaneId::Details)
        .expect("the details pane is visible")
}

fn details_divider_column(terminal: &TestTerminal<App>) -> u16 {
    terminal.driver().layout().commits_details_divider.x
}

fn drag_details_divider_to(terminal: &mut TestTerminal<App>, column: u16) {
    let divider_column = details_divider_column(terminal);
    terminal.mouse_drag((divider_column, BODY_ROW), (column, BODY_ROW));
}

fn details_row_text(terminal: &TestTerminal<App>, row: u16) -> String {
    let pane = details_pane(terminal);
    (pane.x..pane.right())
        .map(|column| terminal.cell(column, row).symbol.clone())
        .collect::<String>()
}

#[test]
fn dragging_the_details_divider_shrinks_past_the_old_floor() {
    let mut terminal = details_terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT);
    assert_eq!(
        details_pane(&terminal).width,
        OLD_DETAILS_FLOOR,
        "the medium screen starts at the default details width"
    );
    drag_details_divider_to(&mut terminal, MEDIUM_WIDTH - DETAILS_MIN_WIDTH - 1);
    let pane = details_pane(&terminal);
    assert_eq!(
        pane.width, DETAILS_MIN_WIDTH,
        "the drag takes the details column down to its minimum"
    );
    assert_eq!(pane.right(), MEDIUM_WIDTH);
    assert_eq!(details_divider_column(&terminal), pane.x - 1);
    assert!(
        terminal.driver().layout().commits.width > 0,
        "the commits column keeps a positive width next to the narrow details column"
    );
    terminal.screenshot("details_resize/medium-minimum");
}

#[test]
fn the_dragged_details_width_beats_the_proportion() {
    let mut terminal = details_terminal(WIDE_WIDTH, WIDE_HEIGHT);
    assert_eq!(
        details_pane(&terminal).width,
        WIDE_LAYOUT_DETAILS_WIDTH,
        "the wide screen starts at the proportional details width"
    );
    drag_details_divider_to(&mut terminal, WIDE_WIDTH - DRAGGED_WIDE_DETAILS_WIDTH - 1);
    assert_eq!(
        details_pane(&terminal).width,
        DRAGGED_WIDE_DETAILS_WIDTH,
        "the dragged width wins over the proportional width"
    );
    assert_eq!(details_pane(&terminal).right(), WIDE_WIDTH);
    terminal.resize(WIDE_WIDTH, WIDE_HEIGHT);
    assert_eq!(
        details_pane(&terminal).width,
        DRAGGED_WIDE_DETAILS_WIDTH,
        "the dragged width survives a redraw at the same size"
    );
    terminal.screenshot("details_resize/wide-dragged");
}

#[test]
fn the_details_width_survives_a_resize() {
    let mut terminal = details_terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT);
    drag_details_divider_to(&mut terminal, MEDIUM_WIDTH - DETAILS_MIN_WIDTH - 1);
    assert_eq!(details_pane(&terminal).width, DETAILS_MIN_WIDTH);
    terminal.resize(COMPACT_WIDTH, COMPACT_HEIGHT);
    assert_eq!(
        terminal.pane_rect(PaneId::Details),
        None,
        "the compact screen hides the details column"
    );
    assert_eq!(terminal.driver().layout().details.width, 0);
    terminal.resize(MEDIUM_WIDTH, MEDIUM_HEIGHT);
    assert_eq!(
        details_pane(&terminal).width,
        DETAILS_MIN_WIDTH,
        "the dragged width comes back with the details column"
    );
    assert_eq!(
        details_divider_column(&terminal),
        MEDIUM_WIDTH - DETAILS_MIN_WIDTH - 1,
        "the divider returns to the dragged position"
    );
    terminal.screenshot("details_resize/after-resize");
}

#[test]
fn a_narrow_details_panel_trims_its_rows_without_overflow() {
    let mut terminal = details_terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT);
    drag_details_divider_to(&mut terminal, MEDIUM_WIDTH - DETAILS_MIN_WIDTH - 1);
    let pane = details_pane(&terminal);
    assert_eq!(pane.width, DETAILS_MIN_WIDTH);
    terminal.assert_no_missing_glyphs();
    let mut separator_rows = 0;
    for row in pane.y..pane.bottom().saturating_sub(BOTTOM_RULE_HEIGHT) {
        let text = details_row_text(&terminal, row);
        assert!(
            text_display_width(&text) <= pane.width,
            "details row {row} renders {text:?} wider than {} columns",
            pane.width
        );
        assert_eq!(
            terminal.cell(pane.x - 1, row).symbol,
            DIVIDER_GLYPH.to_string(),
            "details row {row} keeps its divider on the left edge"
        );
        let last_symbol = terminal.cell(pane.right() - 1, row).symbol.clone();
        assert_eq!(
            last_symbol.chars().count(),
            1,
            "details row {row} ends on a single glyph instead of a split wide character"
        );
        if text.chars().all(|symbol| symbol == DETAILS_SEPARATOR_GLYPH) {
            assert_eq!(
                text.chars().count(),
                usize::from(DETAILS_MIN_WIDTH),
                "the separator row is trimmed to the narrow panel"
            );
            separator_rows += 1;
        }
    }
    assert_eq!(
        separator_rows, 1,
        "the block keeps its single separator row"
    );
    let screen = terminal.screen_text();
    assert!(
        screen.contains(DIRTY_SUBJECT),
        "the narrow panel still shows the commit subject"
    );
    assert!(
        screen.contains(CONTAINING_BRANCH_LABEL),
        "the narrow panel still shows the containing branch label"
    );
    for line in terminal.screen_lines() {
        assert!(
            text_display_width(&line) <= MEDIUM_WIDTH,
            "the narrow details panel does not push the row past the screen edge"
        );
    }
    terminal.screenshot("details_resize/narrow-rows");
}

#[test]
fn the_details_divider_clamps_between_24_and_60_percent() {
    let mut terminal = details_terminal(WIDE_WIDTH, WIDE_HEIGHT);
    drag_details_divider_to(&mut terminal, 0);
    assert_eq!(
        details_pane(&terminal).width,
        details_max_width(WIDE_WIDTH),
        "the drag to the left edge stops at sixty percent of the screen"
    );
    drag_details_divider_to(&mut terminal, WIDE_WIDTH - 1);
    assert_eq!(
        details_pane(&terminal).width,
        DETAILS_MIN_WIDTH,
        "the drag to the right edge stops at the details minimum"
    );
    let layout = terminal.driver().layout();
    assert!(
        layout.commits.width > 0,
        "the commits column never collapses to zero columns"
    );
    assert_eq!(
        layout.commits.right(),
        layout.commits_details_divider.x,
        "the divider sits on the commits right edge"
    );
    assert_eq!(
        layout.commits_details_divider.right(),
        layout.details.x,
        "the details column follows the divider"
    );
    terminal.screenshot("details_resize/wide-maximum");
}

#[test]
fn the_divider_hit_area_follows_the_dragged_position() {
    let mut terminal = details_terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT);
    let old_divider_column = details_divider_column(&terminal);
    drag_details_divider_to(&mut terminal, MEDIUM_WIDTH - DETAILS_MIN_WIDTH - 1);
    let dragged_divider_column = details_divider_column(&terminal);
    assert_ne!(
        dragged_divider_column, old_divider_column,
        "the divider leaves its old column"
    );
    assert_eq!(
        terminal
            .driver()
            .hit_divider(dragged_divider_column, BODY_ROW),
        Some(DividerId::CommitsDetails),
        "the pointer grabs the divider at its dragged column"
    );
    assert_eq!(
        terminal.driver().hit_divider(old_divider_column, BODY_ROW),
        None,
        "the old divider column no longer grabs the pointer"
    );
    terminal.mouse_drag(
        (dragged_divider_column, BODY_ROW),
        (dragged_divider_column - 6, BODY_ROW),
    );
    assert_eq!(
        details_pane(&terminal).width,
        DETAILS_MIN_WIDTH + 6,
        "a second drag starts from the dragged divider column"
    );
    terminal.screenshot("details_resize/hit-area");
}
