use gitdeck_caps::{CapsProfile, ThemeId};
use gitdeck_core::BranchEntry;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::changes::{
    repaint_blank_cell_backgrounds, selection_background, write_row_segments, RowSegment,
};
use crate::glyphs::IconGlyph;
use crate::theme::{theme_palette, Theme};

pub const BRANCHES_RAIL_LABEL: &str = "Branches";
pub const BRANCHES_TITLE: &str = "Branches";
pub const NO_BRANCHES_LABEL: &str = "No branches";
pub const BRANCH_TITLE_ROW_HEIGHT: u16 = 1;
pub const BRANCH_NAME_COLUMN: u16 = 2;
const BRANCH_MARKER_GAP: u16 = 1;

pub struct BranchListView<'a> {
    pub branches: &'a [BranchEntry],
    pub selected_index: Option<usize>,
    pub scroll: usize,
    pub focused: bool,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

pub fn render_branches_rail(area: Rect, active: bool, theme_id: ThemeId, buffer: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let theme = theme_palette(theme_id);
    let rail_style = if active {
        Style::default().fg(theme.accent).bg(theme.background)
    } else {
        Style::default().fg(theme.muted).bg(theme.background)
    };
    buffer.set_style(area, rail_style);
    let letters: Vec<char> = BRANCHES_RAIL_LABEL.chars().collect();
    let letter_count = u16::try_from(letters.len()).unwrap_or(u16::MAX);
    let top = area
        .y
        .saturating_add(area.height.saturating_sub(letter_count) / 2);
    for (offset, letter) in letters.iter().enumerate() {
        let Ok(offset) = u16::try_from(offset) else {
            return;
        };
        let row = top.saturating_add(offset);
        if row >= area.bottom() {
            return;
        }
        buffer.set_stringn(area.x, row, letter.to_string(), 1, rail_style);
    }
}

pub fn branch_list_rows_area(area: Rect) -> Rect {
    Rect::new(
        area.x,
        area.y.saturating_add(BRANCH_TITLE_ROW_HEIGHT),
        area.width,
        area.height.saturating_sub(BRANCH_TITLE_ROW_HEIGHT),
    )
}

pub fn branch_row_rect(list_area: Rect, row_offset: u16) -> Rect {
    Rect::new(
        list_area.x,
        list_area.y.saturating_add(row_offset),
        list_area.width,
        1,
    )
}

pub fn render_branch_list(area: Rect, view: BranchListView<'_>, buffer: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let theme = theme_palette(view.theme);
    let title_style = Style::default().fg(theme.text).bg(theme.background);
    buffer.set_style(area, title_style);
    buffer.set_stringn(
        area.x,
        area.y,
        BRANCHES_TITLE,
        usize::from(area.width),
        title_style,
    );
    let list = branch_list_rows_area(area);
    if list.height == 0 {
        return;
    }
    if view.branches.is_empty() {
        buffer.set_stringn(
            list.x,
            list.y,
            NO_BRANCHES_LABEL,
            usize::from(list.width),
            Style::default().fg(theme.muted).bg(theme.background),
        );
        return;
    }
    let visible_branch_indices = (view.scroll..view.branches.len()).take(usize::from(list.height));
    for (row_offset, branch_index) in visible_branch_indices.enumerate() {
        let Ok(row_offset) = u16::try_from(row_offset) else {
            return;
        };
        let row_y = branch_row_rect(list, row_offset).y;
        let background = selection_background(
            &theme,
            view.focused,
            view.selected_index == Some(branch_index),
            theme.background,
        );
        let segments = branch_row_segments(&view.branches[branch_index], view.caps, &theme);
        write_row_segments(list, row_y, &segments, background, view.caps, buffer);
        repaint_blank_cell_backgrounds(list, row_y, &theme, background, buffer);
    }
}

fn branch_row_segments(branch: &BranchEntry, caps: CapsProfile, theme: &Theme) -> Vec<RowSegment> {
    let mut segments = if branch.is_head {
        vec![
            RowSegment::Text {
                text: IconGlyph::DotFilled.character(caps).to_string(),
                color: theme.accent,
            },
            RowSegment::Pad(BRANCH_MARKER_GAP),
        ]
    } else {
        vec![RowSegment::Pad(BRANCH_NAME_COLUMN)]
    };
    segments.push(RowSegment::Text {
        text: branch.name.clone(),
        color: theme.text,
    });
    segments
}

#[cfg(test)]
mod branch_list_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use gitdeck_core::{BranchEntry, RefKind};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    use super::{
        render_branch_list, render_branches_rail, BranchListView, BRANCHES_RAIL_LABEL,
        BRANCHES_TITLE, NO_BRANCHES_LABEL,
    };

    fn branch(name: &str, kind: RefKind, is_head: bool) -> BranchEntry {
        BranchEntry {
            name: name.to_string(),
            kind,
            is_head,
            upstream: None,
        }
    }

    fn line_text(buffer: &Buffer, row: u16, width: u16) -> String {
        (0..width)
            .map(|column| {
                buffer
                    .cell((column, row))
                    .map(|cell| cell.symbol())
                    .unwrap_or(" ")
            })
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn the_rail_stacks_its_letters_vertically_centered() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 20));
        render_branches_rail(Rect::new(0, 0, 3, 20), true, ThemeId::JbDark, &mut buffer);
        let letters: Vec<String> = (6..14)
            .map(|row| {
                buffer
                    .cell((0, row))
                    .map(|cell| cell.symbol().to_string())
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(letters.concat(), BRANCHES_RAIL_LABEL);
        assert_eq!(
            buffer.cell((0, 13)).map(|cell| cell.symbol().to_string()),
            Some(String::from("s")),
            "the last letter sits two rows above the bottom edge"
        );
        assert_eq!(
            buffer.cell((2, 6)).map(|cell| cell.symbol().to_string()),
            Some(String::from(" ")),
            "the rail keeps its remaining columns blank"
        );
    }

    #[test]
    fn the_list_titles_itself_and_marks_the_current_branch() {
        let branches = vec![
            branch("main", RefKind::LocalBranch, true),
            branch("origin/main", RefKind::RemoteBranch, false),
        ];
        let mut buffer = Buffer::empty(Rect::new(0, 0, 24, 4));
        render_branch_list(
            Rect::new(0, 0, 24, 4),
            BranchListView {
                branches: &branches,
                selected_index: Some(0),
                scroll: 0,
                focused: true,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        assert_eq!(line_text(&buffer, 0, 24), BRANCHES_TITLE);
        assert_eq!(line_text(&buffer, 1, 24), "● main");
        assert_eq!(line_text(&buffer, 2, 24), "  origin/main");
    }

    #[test]
    fn an_empty_branch_list_shows_its_placeholder() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 24, 3));
        render_branch_list(
            Rect::new(0, 0, 24, 3),
            BranchListView {
                branches: &[],
                selected_index: None,
                scroll: 0,
                focused: false,
                caps: CapsProfile::Basic,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        assert_eq!(line_text(&buffer, 1, 24), NO_BRANCHES_LABEL);
    }

    #[test]
    fn the_scroll_offset_skips_the_rows_above_the_window() {
        let branches = vec![
            branch("main", RefKind::LocalBranch, true),
            branch("feature/one", RefKind::LocalBranch, false),
            branch("feature/two", RefKind::LocalBranch, false),
        ];
        let mut buffer = Buffer::empty(Rect::new(0, 0, 24, 4));
        render_branch_list(
            Rect::new(0, 0, 24, 4),
            BranchListView {
                branches: &branches,
                selected_index: Some(2),
                scroll: 2,
                focused: false,
                caps: CapsProfile::Basic,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        assert_eq!(line_text(&buffer, 0, 24), BRANCHES_TITLE);
        assert_eq!(line_text(&buffer, 1, 24), "  feature/two");
        assert_eq!(line_text(&buffer, 2, 24), "");
    }
}
