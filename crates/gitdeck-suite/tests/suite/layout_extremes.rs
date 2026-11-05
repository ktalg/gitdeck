use gitdeck_app::{App, PaneLayout};
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{details_max_width, LOG_RAIL_GAP, LOG_RAIL_WIDTH};

const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const EXTREME_SIZES: [(u16, u16); 2] = [(240, 72), (60, 18)];
const LARGE_SIZES: [(u16, u16); 2] = [(220, 70), (240, 72)];
const ALTERNATING_SIZES: [(u16, u16); 2] = [(240, 72), (60, 18)];
const ALTERNATING_RESIZE_COUNT: usize = 20;
const DRAG_ROW: u16 = 10;
const LEFT_EDGE_COLUMN: u16 = 0;
const RIGHT_EDGE_COLUMN: u16 = 159;
const TOP_BAR_HEIGHT: u16 = 1;
const STATUS_BAR_HEIGHT: u16 = 1;
const ACTIVITY_RAIL_WIDTH: u16 = 4;
const DIVIDER_WIDTH: u16 = 1;
const CHANGES_MIN_WIDTH: u16 = 24;
const CHANGES_MAX_WIDTH: u16 = 56;
const DETAILS_MIN_WIDTH: u16 = 24;
const COMMITS_FLOOR_WIDTH: u16 = 10;

fn terminal_for_size(width: u16, height: u16, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(
        App::new(caps, ThemeId::JbDark),
        width,
        height,
        caps,
        ThemeId::JbDark,
    );
    terminal.resize(width, height);
    terminal
}

fn layout_rects(layout: &PaneLayout) -> [(&'static str, Rect); 11] {
    [
        ("top bar", layout.top_bar),
        ("activity rail", layout.activity_rail),
        ("changes", layout.changes),
        ("changes log divider", layout.changes_log_divider),
        ("log area", layout.log_area),
        ("log tabs", layout.log_tabs),
        ("log filters", layout.log_filters),
        ("commits", layout.commits),
        ("commits details divider", layout.commits_details_divider),
        ("details", layout.details),
        ("status bar", layout.status_bar),
    ]
}

fn assert_layout_inside_screen(terminal: &TestTerminal<App>, width: u16, height: u16) {
    let layout = terminal.driver().layout();
    assert_eq!(
        layout.top_bar.height, TOP_BAR_HEIGHT,
        "{width}x{height} top bar height"
    );
    assert_eq!(
        layout.top_bar.width, width,
        "{width}x{height} top bar width"
    );
    assert_eq!(
        layout.status_bar.height, STATUS_BAR_HEIGHT,
        "{width}x{height} status bar height"
    );
    assert_eq!(
        layout.status_bar.bottom(),
        height,
        "{width}x{height} status bar sits on the last row"
    );
    for (name, rect) in layout_rects(&layout) {
        assert!(
            rect.right() <= width,
            "{width}x{height} {name} ends at column {} beyond {width}",
            rect.right()
        );
        assert!(
            rect.bottom() <= height,
            "{width}x{height} {name} ends at row {} beyond {height}",
            rect.bottom()
        );
    }
    for pane in [PaneId::Changes, PaneId::Log, PaneId::Details] {
        let Some(rect) = terminal.pane_rect(pane) else {
            assert_eq!(
                pane,
                PaneId::Details,
                "{width}x{height} {pane:?} has no rectangle"
            );
            assert_eq!(
                layout.details.width, 0,
                "{width}x{height} only a hidden details column has no rectangle"
            );
            continue;
        };
        assert!(
            rect.right() <= width && rect.bottom() <= height,
            "{width}x{height} {pane:?} rectangle {rect:?} escapes the screen"
        );
    }
}

fn assert_pane_widths_fit_the_log_area(terminal: &TestTerminal<App>, width: u16, height: u16) {
    let layout = terminal.driver().layout();
    assert!(
        (CHANGES_MIN_WIDTH..=CHANGES_MAX_WIDTH).contains(&layout.changes.width),
        "{width}x{height} changes width {} is outside {CHANGES_MIN_WIDTH}..={CHANGES_MAX_WIDTH}",
        layout.changes.width
    );
    assert!(
        layout.details.width <= details_max_width(width),
        "{width}x{height} details width {} exceeds {}",
        layout.details.width,
        details_max_width(width)
    );
    let details_divider_width = if layout.details.width == 0 {
        0
    } else {
        DIVIDER_WIDTH
    };
    assert_eq!(
        layout.commits.width
            + LOG_RAIL_WIDTH
            + LOG_RAIL_GAP
            + layout.details.width
            + details_divider_width,
        layout.log_area.width,
        "{width}x{height} rail, commits, divider and details fill the log area"
    );
    assert_eq!(
        layout.details.width == 0,
        width < 100,
        "{width}x{height} hides the details column below one hundred columns"
    );
    if layout.log_area.width >= DETAILS_MIN_WIDTH + COMMITS_FLOOR_WIDTH + DIVIDER_WIDTH {
        assert!(
            layout.details.width >= DETAILS_MIN_WIDTH,
            "{width}x{height} details width {} drops below {DETAILS_MIN_WIDTH} although \
             the log area is {} columns wide",
            layout.details.width,
            layout.log_area.width
        );
    }
    assert_layout_inside_screen(terminal, width, height);
}

#[test]
fn layout_survives_extreme_sizes() {
    for (width, height) in EXTREME_SIZES {
        for caps in EVERY_CAPS_PROFILE {
            let terminal = terminal_for_size(width, height, caps);
            assert_layout_inside_screen(&terminal, width, height);
            terminal.assert_no_missing_glyphs();
        }
    }
    terminal_for_size(240, 72, CapsProfile::Rich).screenshot("layout_extremes/large-240x72");
    terminal_for_size(60, 18, CapsProfile::Rich).screenshot("layout_extremes/extreme-60x18");
}

#[test]
fn large_size_scales_the_side_panes_with_the_screen() {
    for (width, height) in LARGE_SIZES {
        let terminal = terminal_for_size(width, height, CapsProfile::Rich);
        let layout = terminal.driver().layout();
        let expected_changes_width = (width * 22 / 100).clamp(34, 56);
        let expected_details_width = (width * 28 / 100).clamp(40, 72);
        assert_eq!(
            layout.changes.width, expected_changes_width,
            "{width}x{height}"
        );
        assert_eq!(
            layout.details.width, expected_details_width,
            "{width}x{height}"
        );
        let expected_commits_width = width
            - ACTIVITY_RAIL_WIDTH
            - expected_changes_width
            - DIVIDER_WIDTH
            - expected_details_width
            - DIVIDER_WIDTH
            - LOG_RAIL_WIDTH
            - LOG_RAIL_GAP;
        assert_eq!(
            layout.commits.width, expected_commits_width,
            "{width}x{height} commits absorb the remaining columns"
        );
        assert_layout_inside_screen(&terminal, width, height);
    }
}

#[test]
fn eighty_columns_hand_the_details_column_to_the_commits_list() {
    let terminal = terminal_for_size(80, 24, CapsProfile::Rich);
    let layout = terminal.driver().layout();
    assert_eq!(layout.details.width, 0);
    assert_eq!(layout.commits_details_divider.width, 0);
    assert_eq!(
        layout.commits.width + LOG_RAIL_WIDTH + LOG_RAIL_GAP,
        layout.log_area.width
    );
    assert_layout_inside_screen(&terminal, 80, 24);
}

#[test]
fn divider_drag_then_resize_keeps_widths_clamped() {
    let mut terminal = terminal_for_size(160, 48, CapsProfile::Rich);
    let layout = terminal.driver().layout();
    terminal.mouse_drag(
        (layout.changes_log_divider.x, DRAG_ROW),
        (RIGHT_EDGE_COLUMN, DRAG_ROW),
    );
    assert_pane_widths_fit_the_log_area(&terminal, 160, 48);
    let layout = terminal.driver().layout();
    terminal.mouse_drag(
        (layout.commits_details_divider.x, DRAG_ROW),
        (LEFT_EDGE_COLUMN, DRAG_ROW),
    );
    assert_pane_widths_fit_the_log_area(&terminal, 160, 48);
    terminal.resize(220, 70);
    assert_pane_widths_fit_the_log_area(&terminal, 220, 70);
    terminal.resize(80, 24);
    assert_pane_widths_fit_the_log_area(&terminal, 80, 24);
}

#[test]
fn no_pane_rect_escapes_the_screen_after_ten_resizes() {
    let mut terminal = terminal_for_size(60, 18, CapsProfile::Rich);
    for iteration in 0..ALTERNATING_RESIZE_COUNT {
        let (width, height) = ALTERNATING_SIZES[iteration % ALTERNATING_SIZES.len()];
        terminal.resize(width, height);
        assert_layout_inside_screen(&terminal, width, height);
    }
}
