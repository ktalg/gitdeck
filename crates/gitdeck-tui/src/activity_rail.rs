use gitdeck_caps::CapsProfile;
use gitdeck_core::PaneId;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::glyphs::IconGlyph;
use crate::layout::{rect_contains_cell, ACTIVITY_RAIL_WIDTH};
use crate::theme::Theme;

pub const ACTIVITY_RAIL_ENTRY_HEIGHT: u16 = 3;
pub const ACTIVITY_RAIL_MIN_HEIGHT: u16 = 12;
pub const ACTIVITY_RAIL_ICON_COLUMN: u16 = 1;
pub const ACTIVITY_RAIL_ICON_ROW_OFFSET: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivityRailEntry {
    pub pane: PaneId,
    pub label: &'static str,
    pub icon: IconGlyph,
}

pub const ACTIVITY_RAIL_ENTRIES: [ActivityRailEntry; 3] = [
    ActivityRailEntry {
        pane: PaneId::Changes,
        label: "Changes",
        icon: IconGlyph::DotFilled,
    },
    ActivityRailEntry {
        pane: PaneId::Log,
        label: "Log",
        icon: IconGlyph::Menu,
    },
    ActivityRailEntry {
        pane: PaneId::Details,
        label: "Details",
        icon: IconGlyph::Chevron,
    },
];

pub const ACTIVITY_RAIL_FOOTER_LABELS: [&str; 3] =
    ["Focus: Changes", "Focus: Log", "Focus: Details"];

pub fn activity_rail_entry_index_of(pane: PaneId) -> Option<usize> {
    ACTIVITY_RAIL_ENTRIES
        .iter()
        .position(|entry| entry.pane == pane)
}

pub fn activity_rail_footer_label(pane: PaneId) -> Option<&'static str> {
    activity_rail_entry_index_of(pane).map(|index| ACTIVITY_RAIL_FOOTER_LABELS[index])
}

pub fn activity_rail_is_visible(rail: Rect) -> bool {
    rail.width >= ACTIVITY_RAIL_WIDTH && rail.height >= ACTIVITY_RAIL_MIN_HEIGHT
}

pub fn activity_rail_entry_rects(rail: Rect) -> [Rect; ACTIVITY_RAIL_ENTRIES.len()] {
    let mut rects = [Rect::new(rail.x, rail.y, 0, 0); ACTIVITY_RAIL_ENTRIES.len()];
    if !activity_rail_is_visible(rail) {
        return rects;
    }
    for (index, rect) in rects.iter_mut().enumerate() {
        let offset = ACTIVITY_RAIL_ENTRY_HEIGHT.saturating_mul(u16::try_from(index).unwrap_or(0));
        let height = ACTIVITY_RAIL_ENTRY_HEIGHT.min(rail.height.saturating_sub(offset));
        *rect = Rect::new(rail.x, rail.y.saturating_add(offset), rail.width, height);
    }
    rects
}

pub fn activity_rail_entry_at(rail: Rect, column: u16, row: u16) -> Option<PaneId> {
    activity_rail_entry_rects(rail)
        .into_iter()
        .zip(ACTIVITY_RAIL_ENTRIES)
        .find(|(rect, _)| rect_contains_cell(*rect, column, row))
        .map(|(_, entry)| entry.pane)
}

pub fn activity_rail_icon_rect(rail: Rect, entry: ActivityRailEntry) -> Option<Rect> {
    let index = activity_rail_entry_index_of(entry.pane)?;
    let rect = activity_rail_entry_rects(rail)[index];
    if rect.height <= ACTIVITY_RAIL_ICON_ROW_OFFSET {
        return None;
    }
    let column = rect
        .x
        .saturating_add(ACTIVITY_RAIL_ICON_COLUMN.min(rect.width.saturating_sub(1)));
    Some(Rect::new(
        column,
        rect.y.saturating_add(ACTIVITY_RAIL_ICON_ROW_OFFSET),
        1,
        1,
    ))
}

pub fn render_activity_rail(
    rail: Rect,
    focused_pane: PaneId,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    for (rect, entry) in activity_rail_entry_rects(rail)
        .into_iter()
        .zip(ACTIVITY_RAIL_ENTRIES)
    {
        if rect.width == 0 || rect.height == 0 {
            continue;
        }
        let selected = entry.pane == focused_pane;
        let block_style = if selected {
            Style::default().fg(theme.accent).bg(theme.panel)
        } else {
            Style::default().fg(theme.muted).bg(theme.background)
        };
        fill_rect(rect, block_style, buffer);
        let Some(icon_rect) = activity_rail_icon_rect(rail, entry) else {
            continue;
        };
        put_char(
            buffer,
            icon_rect.x,
            icon_rect.y,
            entry.icon.character(caps),
            block_style,
        );
    }
}

fn fill_rect(rect: Rect, style: Style, buffer: &mut Buffer) {
    for row in rect.y..rect.bottom() {
        for column in rect.x..rect.right() {
            put_char(buffer, column, row, ' ', style);
        }
    }
}

fn put_char(buffer: &mut Buffer, column: u16, row: u16, character: char, style: Style) {
    if let Some(cell) = buffer.cell_mut((column, row)) {
        cell.set_symbol(&character.to_string());
        cell.set_style(style);
    }
}

#[cfg(test)]
mod activity_rail_tests {
    use gitdeck_caps::CapsProfile;
    use gitdeck_core::PaneId;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    use super::{
        activity_rail_entry_at, activity_rail_entry_rects, activity_rail_footer_label,
        activity_rail_icon_rect, render_activity_rail, ACTIVITY_RAIL_ENTRIES,
        ACTIVITY_RAIL_ENTRY_HEIGHT, ACTIVITY_RAIL_ICON_COLUMN, ACTIVITY_RAIL_ICON_ROW_OFFSET,
        ACTIVITY_RAIL_MIN_HEIGHT,
    };
    use crate::theme::theme_palette;

    const RAIL: Rect = Rect::new(0, 1, 4, 40);

    #[test]
    fn the_entries_stack_from_the_top_of_the_rail() {
        let rects = activity_rail_entry_rects(RAIL);
        for (index, rect) in rects.iter().enumerate() {
            assert_eq!(rect.x, RAIL.x);
            assert_eq!(rect.width, RAIL.width);
            assert_eq!(rect.height, ACTIVITY_RAIL_ENTRY_HEIGHT);
            assert_eq!(
                rect.y,
                RAIL.y + ACTIVITY_RAIL_ENTRY_HEIGHT * u16::try_from(index).unwrap(),
                "entry {index} sits below the one above it"
            );
        }
        assert_eq!(rects[0].y, RAIL.y);
    }

    #[test]
    fn a_short_rail_renders_no_entry() {
        let short = Rect::new(0, 1, 4, ACTIVITY_RAIL_MIN_HEIGHT - 1);
        for rect in activity_rail_entry_rects(short) {
            assert_eq!(rect.width, 0, "the short rail shows nothing");
            assert_eq!(rect.height, 0);
        }
        assert_eq!(activity_rail_entry_at(short, 1, 2), None);
        let tall = Rect::new(0, 1, 4, ACTIVITY_RAIL_MIN_HEIGHT);
        assert_eq!(activity_rail_entry_at(tall, 1, 2), Some(PaneId::Changes));
    }

    #[test]
    fn the_icons_sit_on_the_second_row_of_each_block() {
        for entry in ACTIVITY_RAIL_ENTRIES {
            let block = activity_rail_entry_rects(RAIL)
                .into_iter()
                .zip(ACTIVITY_RAIL_ENTRIES)
                .find(|(_, candidate)| candidate.pane == entry.pane)
                .map(|(rect, _)| rect)
                .expect("the entry has a block");
            let icon = activity_rail_icon_rect(RAIL, entry).expect("the entry has an icon cell");
            assert_eq!(icon.y, block.y + ACTIVITY_RAIL_ICON_ROW_OFFSET);
            assert_eq!(icon.x, block.x + ACTIVITY_RAIL_ICON_COLUMN);
            assert_eq!(
                activity_rail_entry_at(RAIL, icon.x, icon.y),
                Some(entry.pane)
            );
        }
    }

    #[test]
    fn the_focused_entry_paints_its_block_with_the_panel_background() {
        for focused in [PaneId::Changes, PaneId::Log, PaneId::Details] {
            let area = Rect::new(0, 0, 4, 41);
            let mut buffer = Buffer::empty(area);
            render_activity_rail(
                RAIL,
                focused,
                CapsProfile::Rich,
                &theme_palette(gitdeck_caps::ThemeId::JbDark),
                &mut buffer,
            );
            let theme = theme_palette(gitdeck_caps::ThemeId::JbDark);
            for (rect, entry) in activity_rail_entry_rects(RAIL)
                .into_iter()
                .zip(ACTIVITY_RAIL_ENTRIES)
            {
                let expected_background = if entry.pane == focused {
                    theme.panel
                } else {
                    theme.background
                };
                for row in rect.y..rect.bottom() {
                    for column in rect.x..rect.right() {
                        assert_eq!(
                            buffer.cell((column, row)).map(|cell| cell.bg),
                            Some(expected_background),
                            "{:?} block at ({column}, {row}) with {focused:?} focused",
                            entry.label
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_entries_carry_labels_and_footer_names() {
        for (entry, footer) in
            ACTIVITY_RAIL_ENTRIES
                .iter()
                .zip(["Focus: Changes", "Focus: Log", "Focus: Details"])
        {
            assert!(!entry.label.is_empty());
            assert_eq!(activity_rail_footer_label(entry.pane), Some(footer));
        }
    }

    #[test]
    fn the_basic_profile_draws_ascii_icons() {
        let area = Rect::new(0, 0, 4, 41);
        let mut buffer = Buffer::empty(area);
        render_activity_rail(
            RAIL,
            PaneId::Changes,
            CapsProfile::Basic,
            &theme_palette(gitdeck_caps::ThemeId::JbDark),
            &mut buffer,
        );
        for entry in ACTIVITY_RAIL_ENTRIES {
            let icon = activity_rail_icon_rect(RAIL, entry).expect("the entry has an icon cell");
            let symbol = buffer
                .cell((icon.x, icon.y))
                .map(|cell| cell.symbol().to_string())
                .unwrap_or_default();
            assert_eq!(symbol, entry.icon.character(CapsProfile::Basic).to_string());
            assert!(symbol.is_ascii());
        }
    }
}
