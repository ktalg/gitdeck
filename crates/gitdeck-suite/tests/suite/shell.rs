use std::path::Path;

use gitdeck_app::App;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{line_glyphs, DividerId, LOG_RAIL_GAP, LOG_RAIL_WIDTH};
use ratatui::style::Modifier;

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const CELL_PIXEL_WIDTH: u32 = 16;
const CELL_PIXEL_HEIGHT: u32 = 32;
const TOP_BAR_HEIGHT: u16 = 1;
const STATUS_BAR_HEIGHT: u16 = 1;
const ACTIVITY_RAIL_WIDTH: u16 = 4;
const CHANGES_DEFAULT_WIDTH: u16 = 34;
const DETAILS_DEFAULT_WIDTH: u16 = 40;
const DIVIDER_COLUMN_OFFSET: u16 = 8;
const DETAILS_RESIZE_STEP: u16 = 10;
const SHORTCUT_SCREEN_ROW: u16 = 5;
const WIDE_LAYOUT_WIDTH: u16 = 220;
const WIDE_LAYOUT_HEIGHT: u16 = 70;
const MEDIUM_LAYOUT_WIDTH: u16 = 120;
const MEDIUM_LAYOUT_HEIGHT: u16 = 32;
const WIDE_CHANGES_WIDTH: u16 = 48;
const WIDE_DETAILS_WIDTH: u16 = 61;
const WIDE_CHANGES_AT_200: u16 = 44;
const WIDE_DETAILS_AT_200: u16 = 56;
const LAYOUT_TABLE: [(u16, u16, u16, u16, u16); 4] = [
    (NARROW_WIDTH, NARROW_HEIGHT, 34, 0, 41),
    (MEDIUM_LAYOUT_WIDTH, MEDIUM_LAYOUT_HEIGHT, 34, 40, 40),
    (
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        35,
        44,
        REFERENCE_WIDTH - ACTIVITY_RAIL_WIDTH - 35 - 1 - 44 - 1,
    ),
    (200, 60, 44, 56, 200 - ACTIVITY_RAIL_WIDTH - 44 - 1 - 56 - 1),
];
const SCREENSHOT_SCENARIOS: [(u16, u16, CapsProfile, ThemeId, &str); 5] = [
    (
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
        "shell/rich-160x48",
    ),
    (
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Standard,
        ThemeId::JbDark,
        "shell/standard-160x48",
    ),
    (
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Basic,
        ThemeId::JbDark,
        "shell/basic-160x48",
    ),
    (
        NARROW_WIDTH,
        NARROW_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
        "shell/rich-80x24",
    ),
    (
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbLight,
        "shell/jb-light-160x48",
    ),
];
const BORDER_COLOR: (u8, u8, u8) = (0x39, 0x3B, 0x40);
const ACCENT_COLOR: (u8, u8, u8) = (0x35, 0x74, 0xF0);
const ASCII_GLYPH_TOP_MARGIN_PIXELS: u32 = 8;
const ASCII_GLYPH_HEIGHT_PIXELS: u32 = 16;
const ASCII_GLYPH_LEFT_MARGIN_PIXELS: u32 = 1;
const STROKE_PIXELS: u32 = 2;
const ALLOWED_GLYPHS: &str = "─│┌┐└┘├┤┬┴┼═║╔╗╚╝╭╮╯╰█▀▄▌▐░▒▓≡←▸▾●○◆✓✗⚠↑↓⌥⚙…";
const BASIC_BORDER_GLYPHS: [char; 2] = ['+', '|'];
const UNICODE_BORDER_GLYPHS: [char; 2] = ['┌', '│'];
const BASIC_FORBIDDEN_ICONS: [char; 2] = ['▾', '●'];

fn shell_terminal(width: u16, height: u16, caps: CapsProfile, theme: ThemeId) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(App::new(caps, theme), width, height, caps, theme);
    terminal.resize(width, height);
    terminal
}

fn scenario_terminal(caps: CapsProfile, theme: ThemeId) -> TestTerminal<App> {
    shell_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps, theme)
}

fn png_dimensions(png_path: &Path) -> (u32, u32) {
    let header = std::fs::read(png_path).expect("screenshot png is readable");
    (
        u32::from_be_bytes([header[16], header[17], header[18], header[19]]),
        u32::from_be_bytes([header[20], header[21], header[22], header[23]]),
    )
}

fn read_png_rgb(png_path: &Path) -> (u32, u32, Vec<u8>) {
    let file = std::fs::File::open(png_path).expect("screenshot png opens");
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().expect("screenshot png header");
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut pixels)
        .expect("screenshot png frame");
    (info.width, info.height, pixels)
}

fn count_color_pixels(
    pixels: &[u8],
    image_width: u32,
    columns: std::ops::Range<u32>,
    rows: std::ops::Range<u32>,
) -> usize {
    let mut painted = 0;
    for row in rows {
        for column in columns.clone() {
            let index = ((row * image_width + column) * 3) as usize;
            let pixel = &pixels[index..index + 3];
            if pixel == [BORDER_COLOR.0, BORDER_COLOR.1, BORDER_COLOR.2]
                || pixel == [ACCENT_COLOR.0, ACCENT_COLOR.1, ACCENT_COLOR.2]
            {
                painted += 1;
            }
        }
    }
    painted
}

fn assert_vertical_border_run_is_continuous(
    pixels: &[u8],
    image_width: u32,
    divider_column: u16,
    first_row: u16,
    last_row: u16,
) {
    let border_columns = u32::from(divider_column) * CELL_PIXEL_WIDTH
        ..u32::from(divider_column + 1) * CELL_PIXEL_WIDTH;
    let first_pixel_row = u32::from(first_row) * CELL_PIXEL_HEIGHT + ASCII_GLYPH_TOP_MARGIN_PIXELS;
    let last_pixel_row = u32::from(last_row) * CELL_PIXEL_HEIGHT
        + ASCII_GLYPH_TOP_MARGIN_PIXELS
        + ASCII_GLYPH_HEIGHT_PIXELS;
    for pixel_row in first_pixel_row..last_pixel_row {
        let painted = count_color_pixels(
            pixels,
            image_width,
            border_columns.clone(),
            pixel_row..pixel_row + 1,
        );
        assert!(
            painted >= STROKE_PIXELS as usize,
            "divider column {divider_column} breaks at pixel row {pixel_row}"
        );
    }
}

fn assert_horizontal_border_run_is_continuous(pixels: &[u8], image_width: u32, row: u16) {
    let border_rows = u32::from(row) * CELL_PIXEL_HEIGHT..u32::from(row + 1) * CELL_PIXEL_HEIGHT;
    for pixel_column in ASCII_GLYPH_LEFT_MARGIN_PIXELS..image_width {
        let painted = count_color_pixels(
            pixels,
            image_width,
            pixel_column..pixel_column + 1,
            border_rows.clone(),
        );
        assert!(
            painted >= STROKE_PIXELS as usize,
            "horizontal border row {row} breaks at pixel column {pixel_column}"
        );
    }
}

fn is_allowed_glyph(character: char) -> bool {
    character == ' ' || character.is_ascii_graphic() || ALLOWED_GLYPHS.contains(character)
}

fn assert_geometry_invariants(terminal: &TestTerminal<App>, width: u16) {
    let layout = terminal.driver().layout();
    assert_eq!(layout.top_bar.height, TOP_BAR_HEIGHT);
    assert_eq!(layout.top_bar.width, width);
    assert_eq!(layout.status_bar.height, STATUS_BAR_HEIGHT);
    assert_eq!(layout.activity_rail.width, ACTIVITY_RAIL_WIDTH);
    assert_eq!(layout.changes.right(), layout.changes_log_divider.x);
    assert_eq!(layout.commits.right(), layout.commits_details_divider.x);
    assert_eq!(layout.details.x, layout.commits_details_divider.right());
    assert_eq!(layout.changes.x, layout.activity_rail.right());
    assert_eq!(layout.log_rail.x, layout.log_area.x);
    assert_eq!(
        layout.commits.x,
        layout.log_rail.right() + LOG_RAIL_GAP,
        "the branches rail keeps one column before the commit list"
    );
    assert_eq!(layout.status_bar.y, layout.activity_rail.bottom());
    if layout.details.width == 0 {
        assert_eq!(layout.commits_details_divider.width, 0);
        assert_eq!(layout.commits.right(), layout.log_area.right());
        return;
    }
    assert_eq!(layout.details.right(), width);
}

#[test]
fn layout_geometry_matches_spec_at_four_sizes() {
    for (width, height, expected_changes, expected_details, expected_commits) in LAYOUT_TABLE {
        let terminal = shell_terminal(width, height, CapsProfile::Rich, ThemeId::JbDark);
        let layout = terminal.driver().layout();
        assert_eq!(
            layout.top_bar,
            Rect::new(0, 0, width, TOP_BAR_HEIGHT),
            "{width}x{height} top bar"
        );
        assert_eq!(
            layout.status_bar,
            Rect::new(0, height - STATUS_BAR_HEIGHT, width, STATUS_BAR_HEIGHT),
            "{width}x{height} status bar"
        );
        assert_eq!(layout.activity_rail.width, ACTIVITY_RAIL_WIDTH);
        assert_eq!(
            layout.changes.width, expected_changes,
            "{width}x{height} changes width"
        );
        assert_eq!(layout.details.width, expected_details, "{width}x{height}");
        assert_eq!(
            layout.commits.width + LOG_RAIL_WIDTH + LOG_RAIL_GAP,
            expected_commits,
            "{width}x{height}"
        );
        assert_geometry_invariants(&terminal, width);
        assert_eq!(layout.log_tabs.y, TOP_BAR_HEIGHT);
        assert_eq!(layout.log_filters.y, TOP_BAR_HEIGHT + 1);
        assert_eq!(layout.commits.y, TOP_BAR_HEIGHT + 2);
    }
}

#[test]
fn eighty_columns_hide_the_details_pane_and_widen_the_commits_pane() {
    let terminal = shell_terminal(
        NARROW_WIDTH,
        NARROW_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    let layout = terminal.driver().layout();
    assert_eq!(
        layout.details.width, 0,
        "screens below 100 columns hide the details column"
    );
    assert_eq!(layout.commits_details_divider.width, 0);
    assert_eq!(
        layout.commits.width + LOG_RAIL_WIDTH + LOG_RAIL_GAP,
        layout.log_area.width
    );
    assert_eq!(terminal.pane_rect(PaneId::Changes), Some(layout.changes));
    assert_eq!(terminal.pane_rect(PaneId::Log), Some(layout.commits));
    assert_eq!(terminal.pane_rect(PaneId::Details), None);
}

#[test]
fn top_bar_shows_project_and_branch_chips() {
    let terminal = scenario_terminal(CapsProfile::Rich, ThemeId::JbDark);
    let top_bar = terminal.screen_lines()[0].clone();
    for needle in ["no repository", "│", "↑", "↓", "?"] {
        assert!(
            top_bar.contains(needle),
            "{needle:?} missing in {top_bar:?}"
        );
    }
    assert!(
        !top_bar.contains("master"),
        "a shell without a repository claims no branch, line {top_bar:?}"
    );
    assert!(
        !top_bar.contains('+'),
        "the dead plus button is gone, line {top_bar:?}"
    );
}

#[test]
fn changes_pane_shows_the_commit_tab_alone() {
    let terminal = scenario_terminal(CapsProfile::Rich, ThemeId::JbDark);
    let layout = terminal.driver().layout();
    let tabs = terminal.screen_lines()[usize::from(layout.changes.y)].clone();
    assert!(tabs.contains("Commit"), "line {tabs:?}");
    assert!(
        !tabs.contains("Shelf"),
        "the inactive shelf tab is gone, line {tabs:?}"
    );
    for column in layout.changes.x..layout.changes.x + 6 {
        let cell = terminal.cell(column, layout.changes.y);
        assert!(
            cell.modifier.contains(Modifier::UNDERLINED),
            "commit tab column {column} is not underlined"
        );
    }
}

#[test]
fn log_area_shows_tab_and_filter_bar() {
    let terminal = scenario_terminal(CapsProfile::Rich, ThemeId::JbDark);
    let layout = terminal.driver().layout();
    let tabs = terminal.screen_lines()[usize::from(layout.log_tabs.y)].clone();
    assert!(tabs.contains("Log: HEAD"), "line {tabs:?}");
    assert!(
        !tabs.contains('+'),
        "the dead add-tab button is gone, line {tabs:?}"
    );
    let filters = terminal.screen_lines()[usize::from(layout.log_filters.y)].clone();
    for label in ["Text or hash", "Cc", "Branch: HEAD"] {
        assert!(filters.contains(label), "{label:?} missing in {filters:?}");
    }
    for removed in [".*", "User", "Date", "Paths"] {
        assert!(
            !filters.contains(removed),
            "{removed:?} is back in {filters:?}"
        );
    }
}

#[test]
fn status_bar_shows_no_repository_and_version() {
    let terminal = scenario_terminal(CapsProfile::Rich, ThemeId::JbDark);
    let status_line = terminal
        .screen_lines()
        .last()
        .cloned()
        .expect("status bar line");
    assert!(
        status_line.contains("no repository"),
        "line {status_line:?}"
    );
    assert!(
        status_line.contains("gitdeck 0.1.0"),
        "line {status_line:?}"
    );
}

#[test]
fn mouse_drag_changes_divider_resizes_neighbours() {
    let mut terminal = scenario_terminal(CapsProfile::Rich, ThemeId::JbDark);
    let before = terminal.driver().layout();
    let divider_column = before.changes_log_divider.x;
    terminal.mouse_drag(
        (divider_column, SHORTCUT_SCREEN_ROW),
        (divider_column + DIVIDER_COLUMN_OFFSET, SHORTCUT_SCREEN_ROW),
    );
    let after = terminal.driver().layout();
    assert_eq!(
        after.changes.width,
        before.changes.width + DIVIDER_COLUMN_OFFSET
    );
    assert_eq!(
        after.commits.width,
        before.commits.width - DIVIDER_COLUMN_OFFSET
    );
    assert_eq!(after.commits.x, before.commits.x + DIVIDER_COLUMN_OFFSET);
    assert_eq!(after.details.width, before.details.width);
    terminal.screenshot("shell/changes-divider-dragged");
}

#[test]
fn mouse_drag_details_divider_resizes_details() {
    let mut terminal = scenario_terminal(CapsProfile::Rich, ThemeId::JbDark);
    let before = terminal.driver().layout();
    let divider_column = before.commits_details_divider.x;
    terminal.mouse_drag(
        (divider_column, SHORTCUT_SCREEN_ROW),
        (divider_column - DETAILS_RESIZE_STEP, SHORTCUT_SCREEN_ROW),
    );
    let after = terminal.driver().layout();
    assert_eq!(
        after.details.width,
        before.details.width + DETAILS_RESIZE_STEP
    );
    assert_eq!(
        after.commits.width,
        before.commits.width - DETAILS_RESIZE_STEP
    );
    assert_eq!(after.details.right(), REFERENCE_WIDTH);
    terminal.screenshot("shell/details-divider-dragged");
}

#[test]
fn divider_widths_clamp_at_bounds() {
    let mut terminal = scenario_terminal(CapsProfile::Rich, ThemeId::JbDark);
    let divider_column = terminal.driver().layout().changes_log_divider.x;
    terminal.mouse_drag(
        (divider_column, SHORTCUT_SCREEN_ROW),
        (0, SHORTCUT_SCREEN_ROW),
    );
    assert_eq!(terminal.driver().layout().changes.width, 24);
    let divider_column = terminal.driver().layout().changes_log_divider.x;
    terminal.mouse_drag(
        (divider_column, SHORTCUT_SCREEN_ROW),
        (REFERENCE_WIDTH - 1, SHORTCUT_SCREEN_ROW),
    );
    assert_eq!(terminal.driver().layout().changes.width, 56);
    let divider_column = terminal.driver().layout().commits_details_divider.x;
    terminal.mouse_drag(
        (divider_column, SHORTCUT_SCREEN_ROW),
        (0, SHORTCUT_SCREEN_ROW),
    );
    assert_eq!(
        terminal.driver().layout().details.width,
        88,
        "the widened details column stops at the commits floor of ten columns"
    );
    let divider_column = terminal.driver().layout().commits_details_divider.x;
    terminal.mouse_drag(
        (divider_column, SHORTCUT_SCREEN_ROW),
        (REFERENCE_WIDTH - 1, SHORTCUT_SCREEN_ROW),
    );
    assert_eq!(terminal.driver().layout().details.width, 24);
}

#[test]
fn wide_layout_gives_the_panels_proportional_widths() {
    let terminal = shell_terminal(
        WIDE_LAYOUT_WIDTH,
        WIDE_LAYOUT_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    let layout = terminal.driver().layout();
    assert_eq!(layout.changes.width, WIDE_CHANGES_WIDTH);
    assert_eq!(layout.details.width, WIDE_DETAILS_WIDTH);
    assert_eq!(
        layout.changes_log_divider.x,
        ACTIVITY_RAIL_WIDTH + WIDE_CHANGES_WIDTH
    );
    assert_eq!(
        layout.commits_details_divider.right() + WIDE_DETAILS_WIDTH,
        WIDE_LAYOUT_WIDTH,
        "details keeps the screen edge"
    );
    assert_eq!(
        layout.log_area.width,
        WIDE_LAYOUT_WIDTH - ACTIVITY_RAIL_WIDTH - WIDE_CHANGES_WIDTH - 1
    );
    assert_eq!(
        layout.commits.width + LOG_RAIL_WIDTH + LOG_RAIL_GAP,
        layout.log_area.width - WIDE_DETAILS_WIDTH - 1,
        "the log column takes the columns the side panels leave"
    );
    assert_geometry_invariants(&terminal, WIDE_LAYOUT_WIDTH);
    terminal.screenshot("shell/proportional-220x70");
}

#[test]
fn narrow_layout_keeps_the_fixed_widths() {
    let terminal = shell_terminal(
        MEDIUM_LAYOUT_WIDTH,
        MEDIUM_LAYOUT_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    let layout = terminal.driver().layout();
    assert_eq!(layout.changes.width, CHANGES_DEFAULT_WIDTH);
    assert_eq!(layout.details.width, DETAILS_DEFAULT_WIDTH);
    assert_eq!(
        layout.log_area.width,
        MEDIUM_LAYOUT_WIDTH - ACTIVITY_RAIL_WIDTH - CHANGES_DEFAULT_WIDTH - 1
    );
    assert_eq!(
        layout.commits.width + LOG_RAIL_WIDTH + LOG_RAIL_GAP,
        layout.log_area.width - DETAILS_DEFAULT_WIDTH - 1
    );
    assert_geometry_invariants(&terminal, MEDIUM_LAYOUT_WIDTH);
    terminal.screenshot("shell/fixed-120x32");
}

#[test]
fn a_dragged_divider_wins_over_the_proportional_width() {
    let mut terminal = shell_terminal(
        WIDE_LAYOUT_WIDTH,
        WIDE_LAYOUT_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    let layout = terminal.driver().layout();
    assert_eq!(layout.changes.width, WIDE_CHANGES_WIDTH);
    let changes_divider_column = layout.changes_log_divider.x;
    terminal.mouse_drag(
        (changes_divider_column, SHORTCUT_SCREEN_ROW),
        (
            changes_divider_column - DIVIDER_COLUMN_OFFSET,
            SHORTCUT_SCREEN_ROW,
        ),
    );
    let dragged_changes_width = terminal.driver().layout().changes.width;
    assert_eq!(
        dragged_changes_width,
        WIDE_CHANGES_WIDTH - DIVIDER_COLUMN_OFFSET
    );
    let details_divider_column = terminal.driver().layout().commits_details_divider.x;
    terminal.mouse_drag(
        (details_divider_column, SHORTCUT_SCREEN_ROW),
        (
            details_divider_column - DETAILS_RESIZE_STEP,
            SHORTCUT_SCREEN_ROW,
        ),
    );
    let dragged_details_width = terminal.driver().layout().details.width;
    assert_eq!(
        dragged_details_width,
        WIDE_DETAILS_WIDTH + DETAILS_RESIZE_STEP
    );
    for (width, height) in [
        (REFERENCE_WIDTH, REFERENCE_HEIGHT),
        (200, 60),
        (MEDIUM_LAYOUT_WIDTH, MEDIUM_LAYOUT_HEIGHT),
    ] {
        terminal.resize(width, height);
        let resized = terminal.driver().layout();
        assert_eq!(
            resized.changes.width, dragged_changes_width,
            "{width}x{height} keeps the dragged changes width"
        );
        assert_eq!(
            resized.changes_log_divider.x,
            ACTIVITY_RAIL_WIDTH + dragged_changes_width,
            "{width}x{height} divider follows the dragged width"
        );
    }
    terminal.resize(WIDE_LAYOUT_WIDTH, WIDE_LAYOUT_HEIGHT);
    assert_eq!(
        terminal.driver().layout().details.width,
        dragged_details_width,
        "the dragged details width survives a resize"
    );
    terminal.screenshot("shell/dragged-widths-220x70");
}

#[test]
fn divider_hit_areas_match_the_rendered_separators() {
    let terminal = shell_terminal(
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    let layout = terminal.driver().layout();
    let vertical = line_glyphs(CapsProfile::Rich).vertical.to_string();
    let glyph_rows_end = layout.activity_rail.bottom() - 1;
    for (divider, divider_id) in [
        (layout.changes_log_divider, DividerId::ChangesLog),
        (layout.commits_details_divider, DividerId::CommitsDetails),
    ] {
        assert_eq!(divider.width, 1, "{divider_id:?} divider column");
        for row in divider.y..glyph_rows_end {
            assert_eq!(
                terminal.cell(divider.x, row).symbol,
                vertical,
                "{divider_id:?} separator column {} row {row}",
                divider.x
            );
        }
        for row in [divider.y, divider.bottom() - 1] {
            for column in divider.x.saturating_sub(2)..=divider.x + 2 {
                let inside_hit_area = column + 1 >= divider.x && column <= divider.x + 1;
                assert_eq!(
                    terminal.driver().hit_divider(column, row),
                    inside_hit_area.then_some(divider_id),
                    "{divider_id:?} hit area at column {column} row {row}"
                );
            }
        }
        assert_eq!(
            terminal.driver().hit_divider(divider.x, divider.y - 1),
            None,
            "{divider_id:?} stays out of the top bar"
        );
    }
}

#[test]
fn tab_cycle_and_shortcuts_focus_expected_pane() {
    let mut terminal = scenario_terminal(CapsProfile::Rich, ThemeId::JbDark);
    assert_eq!(terminal.focused_pane(), PaneId::Changes);
    for expected_pane in [PaneId::Log, PaneId::Details, PaneId::Changes] {
        terminal.send_keys("tab");
        assert_eq!(terminal.focused_pane(), expected_pane);
    }
    for expected_pane in [PaneId::Details, PaneId::Log, PaneId::Changes] {
        terminal.send_keys("backtab");
        assert_eq!(terminal.focused_pane(), expected_pane);
    }
    terminal.send_keys("ctrl+k");
    assert_eq!(terminal.focused_pane(), PaneId::Changes);
    terminal.send_keys("alt+9");
    assert_eq!(terminal.focused_pane(), PaneId::Log);
}

#[test]
fn basic_profile_uses_ascii_borders_and_icons() {
    let terminal = scenario_terminal(CapsProfile::Basic, ThemeId::JbDark);
    let screen_text = terminal.screen_text();
    for glyph in BASIC_BORDER_GLYPHS {
        assert!(screen_text.contains(glyph), "{glyph:?} missing");
    }
    for glyph in UNICODE_BORDER_GLYPHS {
        assert!(!screen_text.contains(glyph), "{glyph:?} must be downgraded");
    }
    for glyph in BASIC_FORBIDDEN_ICONS {
        assert!(!screen_text.contains(glyph), "{glyph:?} must be downgraded");
    }
    assert!(screen_text.contains("Log: HEAD"));
    assert!(screen_text.contains("Text or hash"));
}

#[test]
fn screenshots_cover_profiles_sizes_themes() {
    for (width, height, caps, theme, name) in SCREENSHOT_SCENARIOS {
        let terminal = shell_terminal(width, height, caps, theme);
        let report = terminal.screenshot(name);
        assert_eq!(
            png_dimensions(&report.png_path),
            (
                u32::from(width) * CELL_PIXEL_WIDTH,
                u32::from(height) * CELL_PIXEL_HEIGHT
            ),
            "{name} png dimensions"
        );
        assert!(report.missing_glyphs.is_empty(), "{name} missing glyphs");
        for artifact_path in [&report.text_path, &report.ansi_path] {
            assert!(
                artifact_path.is_file(),
                "{} is missing",
                artifact_path.display()
            );
        }
    }
}

#[test]
fn ascii_border_runs_render_continuous() {
    let terminal = scenario_terminal(CapsProfile::Basic, ThemeId::JbDark);
    let layout = terminal.driver().layout();
    let report = terminal.screenshot("shell/basic-160x48");
    let (image_width, image_height, pixels) = read_png_rgb(&report.png_path);
    assert_eq!(image_width, u32::from(REFERENCE_WIDTH) * CELL_PIXEL_WIDTH);
    assert_eq!(
        image_height,
        u32::from(REFERENCE_HEIGHT) * CELL_PIXEL_HEIGHT
    );
    let rule_row = layout.activity_rail.bottom() - 1;
    for divider in [layout.changes_log_divider, layout.commits_details_divider] {
        assert_vertical_border_run_is_continuous(
            &pixels,
            image_width,
            divider.x,
            divider.y,
            rule_row,
        );
    }
    assert_horizontal_border_run_is_continuous(&pixels, image_width, rule_row);
}

#[test]
fn no_glyph_outside_allowlist() {
    for (width, height, caps, theme, name) in SCREENSHOT_SCENARIOS {
        let terminal = shell_terminal(width, height, caps, theme);
        terminal.screenshot(name);
        for row in 0..height {
            for column in 0..width {
                for character in terminal.cell(column, row).symbol.chars() {
                    assert!(
                        is_allowed_glyph(character),
                        "{name}: glyph {character:?} at ({column}, {row}) is outside the allowlist"
                    );
                }
            }
        }
        assert!(
            !terminal.screen_text().trim().is_empty(),
            "{name} rendered nothing"
        );
    }
}

#[test]
fn resize_keeps_layout_consistent() {
    let mut terminal = shell_terminal(
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    assert_geometry_invariants(&terminal, REFERENCE_WIDTH);
    terminal.resize(NARROW_WIDTH, NARROW_HEIGHT);
    assert_geometry_invariants(&terminal, NARROW_WIDTH);
    assert_eq!(terminal.driver().layout().details.width, 0);
    assert_eq!(
        terminal.driver().layout().commits.width + LOG_RAIL_WIDTH + LOG_RAIL_GAP,
        terminal.driver().layout().log_area.width
    );
    terminal.resize(200, 60);
    assert_geometry_invariants(&terminal, 200);
    assert_eq!(
        terminal.driver().layout().changes.width,
        WIDE_CHANGES_AT_200
    );
    assert_eq!(
        terminal.driver().layout().details.width,
        WIDE_DETAILS_AT_200
    );
    terminal.screenshot("shell/resized-200x60");
}
