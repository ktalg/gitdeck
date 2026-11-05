use gitdeck_app::App;
use gitdeck_core::{InputEvent, MouseButton, MouseInput, MouseInputKind, PaneId};
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    activity_rail_icon_rect, help_overlay_layout, help_overlay_visible_row_count, theme_palette,
    wrap_help_rows, ACTIVITY_RAIL_ENTRIES, ACTIVITY_RAIL_ICON_COLUMN, ACTIVITY_RAIL_MIN_HEIGHT,
    HELP_OVERLAY_MORE_ASCII_LABEL, HELP_OVERLAY_MORE_LABEL, HELP_OVERLAY_TITLE,
};
use ratatui::layout::Rect;

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const SHORT_SCREEN_HEIGHT: u16 = ACTIVITY_RAIL_MIN_HEIGHT + 1;

fn rail_terminal(width: u16, height: u16, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::Dirty,
        width,
        height,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the fixture repository opens")
        },
    );
    terminal.resize(width, height);
    terminal
}

fn medium_terminal(caps: CapsProfile) -> TestTerminal<App> {
    rail_terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT, caps)
}

fn narrow_terminal(caps: CapsProfile) -> TestTerminal<App> {
    rail_terminal(NARROW_WIDTH, NARROW_HEIGHT, caps)
}

fn move_pointer(terminal: &mut TestTerminal<App>, column: u16, row: u16) {
    terminal
        .driver_mut()
        .handle_event(InputEvent::Mouse(MouseInput {
            kind: MouseInputKind::Move,
            button: MouseButton::None,
            column,
            row,
            modifiers: Default::default(),
            click_count: 1,
        }));
    terminal.tick();
}

fn rail_icon_cell(terminal: &TestTerminal<App>, entry_index: usize) -> (u16, u16) {
    let rail = terminal.driver().layout().activity_rail;
    let icon = activity_rail_icon_rect(rail, ACTIVITY_RAIL_ENTRIES[entry_index])
        .expect("the entry has an icon cell");
    (icon.x, icon.y)
}

fn rail_body_bottom(rail: Rect) -> u16 {
    rail.bottom()
        .saturating_sub(gitdeck_tui::BOTTOM_RULE_HEIGHT)
}

fn registry_rows() -> Vec<(&'static str, &'static str)> {
    gitdeck_app::KEY_BINDINGS
        .iter()
        .map(|binding| (binding.keys, binding.action))
        .collect()
}

fn help_rows_on_80_columns() -> Vec<String> {
    let layout = help_overlay_layout(Rect::new(0, 0, NARROW_WIDTH, NARROW_HEIGHT))
        .expect("the overlay fits the narrow screen");
    wrap_help_rows(&registry_rows(), layout.rows.width)
}

fn screen_text_contains_row(terminal: &TestTerminal<App>, row: &str) -> bool {
    terminal
        .screen_lines()
        .iter()
        .any(|line| line.contains(row.trim_end()))
}

#[test]
fn the_activity_rail_lists_three_focus_entries() {
    let theme = theme_palette(ThemeId::JbDark);
    for caps in EVERY_CAPS_PROFILE {
        let terminal = medium_terminal(caps);
        let rail = terminal.driver().layout().activity_rail;
        for (index, entry) in ACTIVITY_RAIL_ENTRIES.iter().enumerate() {
            let (column, row) = rail_icon_cell(&terminal, index);
            assert_eq!(column, rail.x + ACTIVITY_RAIL_ICON_COLUMN);
            assert_eq!(row, rail.y + 1 + 3 * u16::try_from(index).unwrap());
            let cell = terminal.cell(column, row);
            assert_eq!(
                cell.symbol,
                entry.icon.character(caps).to_string(),
                "{caps:?} {:?} icon",
                entry.label
            );
            let selected = entry.pane == PaneId::Changes;
            assert_eq!(
                cell.foreground,
                if selected { theme.accent } else { theme.muted },
                "{caps:?} {:?} icon colour",
                entry.label
            );
            assert_eq!(
                cell.background,
                if selected {
                    theme.panel
                } else {
                    theme.background
                },
                "{caps:?} {:?} block background",
                entry.label
            );
        }
        let (column, row) = rail_icon_cell(&terminal, 1);
        assert_eq!(
            terminal.cell(column.saturating_sub(1), row).background,
            theme.background,
            "{caps:?} the unfocused block keeps the background"
        );
    }
}

#[test]
fn clicking_a_rail_entry_focuses_that_pane() {
    let mut terminal = medium_terminal(CapsProfile::Rich);
    let theme = theme_palette(ThemeId::JbDark);
    assert_eq!(terminal.focused_pane(), PaneId::Changes);
    for (entry_index, expected_pane) in
        [(1, PaneId::Log), (2, PaneId::Details), (0, PaneId::Changes)]
    {
        let (column, row) = rail_icon_cell(&terminal, entry_index);
        terminal.mouse_click(column, row);
        assert_eq!(
            terminal.focused_pane(),
            expected_pane,
            "clicking rail entry {entry_index} focuses {expected_pane:?}"
        );
        assert_eq!(terminal.cell(column, row).foreground, theme.accent);
        assert_eq!(terminal.cell(column, row).background, theme.panel);
    }
    terminal.screenshot("activity_rail/click-changes");
}

#[test]
fn alt_1_alt_2_alt_3_focus_the_panes() {
    let mut terminal = medium_terminal(CapsProfile::Rich);
    terminal.send_keys("alt+2");
    assert_eq!(terminal.focused_pane(), PaneId::Log);
    terminal.send_keys("alt+3");
    assert_eq!(terminal.focused_pane(), PaneId::Details);
    terminal.send_keys("alt+1");
    assert_eq!(terminal.focused_pane(), PaneId::Changes);
    terminal.screenshot("activity_rail/alt-shortcuts");
}

#[test]
fn hovering_a_rail_entry_shows_its_name_in_the_footer() {
    let mut terminal = medium_terminal(CapsProfile::Rich);
    let status_row = usize::from(MEDIUM_HEIGHT - 1);
    for (entry_index, footer_label) in [
        (0, "Focus: Changes"),
        (1, "Focus: Log"),
        (2, "Focus: Details"),
    ] {
        let (column, row) = rail_icon_cell(&terminal, entry_index);
        move_pointer(&mut terminal, column, row);
        let status_line = terminal.screen_lines()[status_row].clone();
        assert!(
            status_line.contains(footer_label),
            "the footer names {footer_label:?}, the status line reads {status_line:?}"
        );
    }
    move_pointer(&mut terminal, MEDIUM_WIDTH / 2, MEDIUM_HEIGHT / 2);
    let cleared_line = terminal.screen_lines()[status_row].clone();
    assert!(
        !cleared_line.contains("Focus: "),
        "the footer drops the entry name when the pointer leaves, got {cleared_line:?}"
    );
    terminal.screenshot("activity_rail/footer-hint-cleared");
    let (column, row) = rail_icon_cell(&terminal, 1);
    move_pointer(&mut terminal, column, row);
    terminal.screenshot("activity_rail/footer-hint-log");
}

#[test]
fn the_rail_is_hidden_on_a_short_screen_but_the_shortcuts_still_work() {
    let mut terminal = rail_terminal(MEDIUM_WIDTH, SHORT_SCREEN_HEIGHT, CapsProfile::Rich);
    let rail = terminal.driver().layout().activity_rail;
    assert!(rail.height < ACTIVITY_RAIL_MIN_HEIGHT);
    for entry in ACTIVITY_RAIL_ENTRIES {
        assert!(activity_rail_icon_rect(rail, entry).is_none());
    }
    for row in rail.y..rail_body_bottom(rail) {
        for column in rail.x..rail.right() {
            assert_eq!(
                terminal.cell(column, row).symbol,
                " ",
                "rail cell ({column}, {row}) stays blank while the rail is hidden"
            );
        }
    }
    terminal.send_keys("alt+2");
    assert_eq!(terminal.focused_pane(), PaneId::Log);
    terminal.send_keys("alt+3");
    assert_eq!(terminal.focused_pane(), PaneId::Details);
    terminal.send_keys("alt+1");
    assert_eq!(terminal.focused_pane(), PaneId::Changes);
    terminal.screenshot("activity_rail/hidden-short-screen");
}

#[test]
fn the_help_overlay_lists_the_binding_registry() {
    let mut terminal = narrow_terminal(CapsProfile::Rich);
    terminal.send_keys("?");
    let rows = help_rows_on_80_columns();
    let visible_rows = help_overlay_visible_row_count(Rect::new(0, 0, NARROW_WIDTH, NARROW_HEIGHT));
    assert!(visible_rows > 1);
    assert!(rows.len() > visible_rows, "the registry scrolls");
    assert!(terminal.screen_text().contains(HELP_OVERLAY_TITLE));
    for row in rows.iter().take(visible_rows) {
        assert!(
            screen_text_contains_row(&terminal, row),
            "the first page shows {row:?}, screen text is:\n{}",
            terminal.screen_text()
        );
    }
    for _ in 0..rows.len() {
        terminal.send_keys("j");
    }
    for row in rows.iter().skip(rows.len() - visible_rows) {
        assert!(
            screen_text_contains_row(&terminal, row),
            "the last page shows {row:?}, screen text is:\n{}",
            terminal.screen_text()
        );
    }
    terminal.screenshot("activity_rail/help-overlay-scrolled");
    for _ in 0..rows.len() {
        terminal.send_keys("k");
    }
    assert!(screen_text_contains_row(&terminal, &rows[0]));
    terminal.screenshot("activity_rail/help-overlay-80x24");
}

#[test]
fn the_help_overlay_swallows_keys_and_closes_with_escape() {
    let mut terminal = narrow_terminal(CapsProfile::Rich);
    terminal.send_keys("?");
    assert!(terminal.screen_text().contains(HELP_OVERLAY_TITLE));
    terminal.send_keys("alt+2");
    assert_eq!(
        terminal.focused_pane(),
        PaneId::Changes,
        "the overlay swallows the focus shortcut"
    );
    terminal.send_keys("tab");
    assert_eq!(
        terminal.focused_pane(),
        PaneId::Changes,
        "the overlay swallows tab"
    );
    terminal.send_keys("esc");
    assert!(
        !terminal.screen_text().contains(HELP_OVERLAY_TITLE),
        "escape closes the overlay"
    );
    assert_eq!(terminal.focused_pane(), PaneId::Changes);
    terminal.send_keys("?");
    terminal.send_keys("?");
    assert!(
        !terminal.screen_text().contains(HELP_OVERLAY_TITLE),
        "a second question mark closes the overlay"
    );
    terminal.send_keys("tab");
    assert_eq!(terminal.focused_pane(), PaneId::Log);
}

#[test]
fn the_help_overlay_fits_an_80x24_screen() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = narrow_terminal(caps);
        terminal.send_keys("?");
        let layout = help_overlay_layout(Rect::new(0, 0, NARROW_WIDTH, NARROW_HEIGHT))
            .expect("the overlay fits the narrow screen");
        assert!(layout.frame.right() <= NARROW_WIDTH);
        assert!(layout.frame.bottom() <= NARROW_HEIGHT);
        assert!(terminal.screen_text().contains(HELP_OVERLAY_TITLE));
        let rows = help_rows_on_80_columns();
        for (row_index, row) in rows
            .iter()
            .take(usize::from(layout.rows.height))
            .enumerate()
        {
            let screen_row = layout.rows.y + u16::try_from(row_index).unwrap();
            let content: String = (layout.rows.x..layout.rows.right())
                .map(|column| terminal.cell(column, screen_row).symbol)
                .collect::<Vec<String>>()
                .concat();
            let marker = match caps {
                CapsProfile::Basic => HELP_OVERLAY_MORE_ASCII_LABEL,
                CapsProfile::Rich | CapsProfile::Standard => HELP_OVERLAY_MORE_LABEL,
            };
            let content = content.trim_end();
            let content = content
                .strip_suffix(marker)
                .map(str::trim_end)
                .unwrap_or(content);
            assert_eq!(
                content,
                row.trim_end(),
                "{caps:?} row {screen_row} carries the wrapped registry row"
            );
            for column in layout.frame.x..layout.frame.right() {
                let symbol = terminal.cell(column, screen_row).symbol;
                if caps == CapsProfile::Basic {
                    assert!(
                        symbol.is_ascii(),
                        "{caps:?} cell ({column}, {screen_row}) holds {symbol:?}"
                    );
                }
            }
        }
        terminal.screenshot(format!("activity_rail/help-80x24-{caps:?}").as_str());
    }
}

#[test]
fn the_activity_rail_never_shows_unlabeled_glyphs() {
    for caps in EVERY_CAPS_PROFILE {
        for (width, height) in [
            (REFERENCE_WIDTH, REFERENCE_HEIGHT),
            (MEDIUM_WIDTH, MEDIUM_HEIGHT),
            (NARROW_WIDTH, NARROW_HEIGHT),
            (MEDIUM_WIDTH, SHORT_SCREEN_HEIGHT),
        ] {
            let terminal = rail_terminal(width, height, caps);
            let rail = terminal.driver().layout().activity_rail;
            let mut labelled_cells: Vec<(u16, u16, String)> = Vec::new();
            for entry in ACTIVITY_RAIL_ENTRIES {
                let Some(icon) = activity_rail_icon_rect(rail, entry) else {
                    continue;
                };
                labelled_cells.push((icon.x, icon.y, entry.icon.character(caps).to_string()));
            }
            for row in rail.y..rail_body_bottom(rail) {
                for column in rail.x..rail.right() {
                    let symbol = terminal.cell(column, row).symbol;
                    let labelled = labelled_cells.iter().any(|(icon_column, icon_row, glyph)| {
                        *icon_column == column && *icon_row == row && *glyph == symbol
                    });
                    assert!(
                        labelled || symbol == " ",
                        "{caps:?} at {width}x{height}: rail cell ({column}, {row}) holds the unlabelled glyph {symbol:?}"
                    );
                }
            }
            let expected_icons = if height == SHORT_SCREEN_HEIGHT { 0 } else { 3 };
            assert_eq!(
                labelled_cells.len(),
                expected_icons,
                "{caps:?} at {width}x{height} draws {expected_icons} icons"
            );
            terminal.screenshot(&format!("activity_rail/rail-{width}x{height}-{caps:?}"));
        }
    }
    medium_terminal(CapsProfile::Rich).screenshot("activity_rail/entries-120x32");
}
