use gitdeck_caps::{CapsProfile, ThemeId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::changes::box_glyphs;
use crate::dialog_frame::{dialog_panel_style, fill_rect};
use crate::shell::text_display_width;
use crate::theme::{theme_palette, Theme};

pub const BRANCH_FILTER_HEAD_LABEL: &str = "HEAD";
pub const BRANCH_FILTER_CURRENT_BRANCH_PREFIX: &str = "* ";
pub const BRANCH_FILTER_DROPDOWN_MAX_VISIBLE_ROWS: usize = 10;
pub const BRANCH_FILTER_DROPDOWN_LABEL_PADDING: u16 = 4;
pub const BRANCH_FILTER_DROPDOWN_ROW_INSET: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BranchFilterDropdownLayout {
    pub frame: Rect,
    pub rows: Rect,
}

impl BranchFilterDropdownLayout {
    pub fn visible_row_count(self) -> usize {
        usize::from(self.rows.height)
    }

    pub fn row_rect(self, visible_row: usize) -> Option<Rect> {
        if visible_row >= self.visible_row_count() {
            return None;
        }
        let offset = u16::try_from(visible_row).ok()?;
        Some(Rect::new(
            self.rows.x,
            self.rows.y.saturating_add(offset),
            self.rows.width,
            1,
        ))
    }

    pub fn row_index_at(self, column: u16, row: u16) -> Option<usize> {
        if column < self.rows.x
            || column >= self.rows.right()
            || row < self.rows.y
            || row >= self.rows.bottom()
        {
            return None;
        }
        Some(usize::from(row - self.rows.y))
    }
}

pub fn branch_filter_dropdown_layout(
    anchor: Rect,
    item_labels: &[String],
    area: Rect,
) -> Option<BranchFilterDropdownLayout> {
    if item_labels.is_empty() || area.width == 0 || area.height == 0 {
        return None;
    }
    let visible_row_count = item_labels
        .len()
        .min(BRANCH_FILTER_DROPDOWN_MAX_VISIBLE_ROWS);
    let visible_row_count = u16::try_from(visible_row_count).ok()?;
    let longest_label_width = item_labels
        .iter()
        .map(|label| text_display_width(label))
        .max()
        .unwrap_or(0);
    let width = anchor
        .width
        .max(longest_label_width.saturating_add(BRANCH_FILTER_DROPDOWN_LABEL_PADDING))
        .min(area.width);
    let height = visible_row_count.saturating_add(2);
    if height > area.height {
        return None;
    }
    let frame = Rect::new(
        anchor.x.min(area.right().saturating_sub(width)),
        anchor.bottom().min(area.bottom().saturating_sub(height)),
        width,
        height,
    );
    Some(BranchFilterDropdownLayout {
        frame,
        rows: Rect::new(
            frame.x.saturating_add(BRANCH_FILTER_DROPDOWN_ROW_INSET),
            frame.y.saturating_add(1),
            frame
                .width
                .saturating_sub(BRANCH_FILTER_DROPDOWN_ROW_INSET * 2),
            visible_row_count,
        ),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BranchFilterDropdownView<'a> {
    pub items: &'a [String],
    pub selected_index: usize,
    pub scroll: usize,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

pub fn render_branch_filter_dropdown(
    anchor: Rect,
    area: Rect,
    view: BranchFilterDropdownView<'_>,
    buffer: &mut Buffer,
) {
    let Some(layout) = branch_filter_dropdown_layout(anchor, view.items, area) else {
        return;
    };
    let theme = theme_palette(view.theme);
    fill_rect(layout.frame, dialog_panel_style(&theme), buffer);
    draw_dropdown_border(layout.frame, view.caps, &theme, buffer);
    for visible_row in 0..layout.visible_row_count() {
        let Some(item_index) = view.scroll.checked_add(visible_row) else {
            return;
        };
        let Some(label) = view.items.get(item_index) else {
            return;
        };
        let Some(row) = layout.row_rect(visible_row) else {
            return;
        };
        let style = branch_filter_row_style(item_index == view.selected_index, &theme);
        fill_rect(row, style, buffer);
        buffer.set_stringn(row.x, row.y, label, usize::from(row.width), style);
    }
}

fn branch_filter_row_style(selected: bool, theme: &Theme) -> Style {
    if selected {
        return Style::default().fg(theme.text).bg(theme.accent);
    }
    Style::default().fg(theme.text).bg(theme.panel)
}

fn draw_dropdown_border(frame: Rect, caps: CapsProfile, theme: &Theme, buffer: &mut Buffer) {
    let glyphs = box_glyphs(caps);
    let style = Style::default().fg(theme.border).bg(theme.panel);
    let bottom_row = frame.bottom().saturating_sub(1);
    let right_column = frame.right().saturating_sub(1);
    for column in frame.x..frame.right() {
        put_glyph(buffer, column, frame.y, glyphs.horizontal, style);
        put_glyph(buffer, column, bottom_row, glyphs.horizontal, style);
    }
    for row in frame.y..frame.bottom() {
        put_glyph(buffer, frame.x, row, glyphs.vertical, style);
        put_glyph(buffer, right_column, row, glyphs.vertical, style);
    }
    for (column, row, glyph) in [
        (frame.x, frame.y, glyphs.top_left),
        (right_column, frame.y, glyphs.top_right),
        (frame.x, bottom_row, glyphs.bottom_left),
        (right_column, bottom_row, glyphs.bottom_right),
    ] {
        put_glyph(buffer, column, row, glyph, style);
    }
}

fn put_glyph(buffer: &mut Buffer, column: u16, row: u16, glyph: char, style: Style) {
    if let Some(cell) = buffer.cell_mut((column, row)) {
        cell.set_symbol(&glyph.to_string());
        cell.set_style(style);
    }
}

#[cfg(test)]
mod branch_filter_dropdown_tests {
    use gitdeck_caps::CapsProfile;
    use ratatui::layout::Rect;

    use super::{branch_filter_dropdown_layout, BranchFilterDropdownLayout};

    fn labels(count: usize) -> Vec<String> {
        (0..count).map(|index| format!("branch-{index}")).collect()
    }

    fn layout_for(anchor: Rect, item_count: usize) -> BranchFilterDropdownLayout {
        branch_filter_dropdown_layout(anchor, &labels(item_count), Rect::new(0, 0, 80, 24))
            .expect("the dropdown fits the screen")
    }

    #[test]
    fn the_dropdown_sits_directly_below_the_anchor() {
        let layout = layout_for(Rect::new(20, 2, 11, 1), 3);
        assert_eq!(layout.frame.x, 20);
        assert_eq!(layout.frame.y, 3);
        assert_eq!(layout.frame.height, 5, "one border row above and below");
        assert_eq!(layout.visible_row_count(), 3);
    }

    #[test]
    fn the_dropdown_grows_to_the_longest_label_plus_padding() {
        let short_anchor = Rect::new(0, 2, 11, 1);
        let items = vec![String::from("HEAD"), String::from("feature/a-long-branch")];
        let layout = branch_filter_dropdown_layout(short_anchor, &items, Rect::new(0, 0, 80, 24))
            .expect("the dropdown fits");
        assert_eq!(
            layout.frame.width, 25,
            "the longest label plus four columns of padding"
        );
    }

    #[test]
    fn more_than_ten_entries_cap_the_visible_rows() {
        let layout = layout_for(Rect::new(0, 2, 11, 1), 25);
        assert_eq!(layout.visible_row_count(), 10);
        assert_eq!(layout.frame.height, 12);
        assert!(layout.row_rect(10).is_none());
    }

    #[test]
    fn a_dropdown_near_the_right_edge_shifts_left_to_fit() {
        let layout = branch_filter_dropdown_layout(
            Rect::new(74, 2, 11, 1),
            &[String::from("HEAD")],
            Rect::new(0, 0, 80, 24),
        )
        .expect("the dropdown fits");
        assert_eq!(layout.frame.right(), 80);
    }

    #[test]
    fn rows_map_back_to_their_visible_index() {
        let layout = layout_for(Rect::new(10, 2, 11, 1), 4);
        assert_eq!(layout.row_index_at(11, 4), Some(0));
        assert_eq!(layout.row_index_at(11, 7), Some(3));
        assert_eq!(layout.row_index_at(11, 8), None);
        assert_eq!(layout.row_index_at(9, 4), None);
    }

    #[test]
    fn the_dropdown_keeps_the_border_glyphs_of_the_requested_caps() {
        let mut buffer = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 80, 24));
        super::render_branch_filter_dropdown(
            Rect::new(10, 2, 11, 1),
            Rect::new(0, 0, 80, 24),
            super::BranchFilterDropdownView {
                items: &[String::from("HEAD"), String::from("main")],
                selected_index: 1,
                scroll: 0,
                caps: CapsProfile::Rich,
                theme: gitdeck_caps::ThemeId::JbDark,
            },
            &mut buffer,
        );
        assert_eq!(buffer.cell((10, 3)).expect("corner").symbol(), "┌");
        assert_eq!(buffer.cell((11, 4)).expect("row").symbol(), "H");
        assert_eq!(buffer.cell((11, 5)).expect("row").symbol(), "m");
    }
}
