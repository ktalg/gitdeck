use gitdeck_caps::{CapsProfile, ThemeId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::dialog_frame::{
    centered_dialog_frame, dialog_content_columns, dialog_panel_style, draw_dialog_border,
    draw_text_row, fill_rect,
};
use crate::shell::text_display_width;
use crate::theme::theme_palette;

pub const HELP_OVERLAY_TITLE: &str = "Keys";
pub const HELP_OVERLAY_MAX_WIDTH: u16 = 72;
pub const HELP_OVERLAY_MAX_HEIGHT: u16 = 22;
pub const HELP_OVERLAY_KEYS_GAP: &str = "  ";
pub const HELP_OVERLAY_PAGE_ROWS: usize = 5;
pub const HELP_OVERLAY_MORE_LABEL: &str = "↓ more";
pub const HELP_OVERLAY_MORE_ASCII_LABEL: &str = "v more";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HelpOverlayLayout {
    pub frame: Rect,
    pub rows: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HelpOverlayView<'a> {
    pub rows: &'a [String],
    pub scroll: usize,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

pub fn help_overlay_layout(area: Rect) -> Option<HelpOverlayLayout> {
    let frame = centered_dialog_frame(area, HELP_OVERLAY_MAX_WIDTH, HELP_OVERLAY_MAX_HEIGHT)?;
    let (content_x, content_width) = dialog_content_columns(frame);
    Some(HelpOverlayLayout {
        frame,
        rows: Rect::new(
            content_x,
            frame.y.saturating_add(1),
            content_width,
            frame.height.saturating_sub(2),
        ),
    })
}

pub fn help_overlay_visible_row_count(area: Rect) -> usize {
    help_overlay_layout(area)
        .map(|layout| usize::from(layout.rows.height))
        .unwrap_or(0)
}

pub fn wrap_help_rows(bindings: &[(&str, &str)], width: u16) -> Vec<String> {
    let mut rows = Vec::new();
    for (keys, action) in bindings {
        let line = format!("{keys}{HELP_OVERLAY_KEYS_GAP}{action}");
        rows.extend(wrap_help_line(&line, width));
    }
    rows
}

fn wrap_help_line(line: &str, width: u16) -> Vec<String> {
    let limit = width.max(1);
    let mut rows = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
            continue;
        }
        let candidate = format!("{current} {word}");
        if text_display_width(&candidate) > limit {
            rows.push(std::mem::take(&mut current));
            current.push_str(word);
            continue;
        }
        current = candidate;
    }
    if !current.is_empty() {
        rows.push(current);
    }
    rows
}

pub fn render_help_overlay(area: Rect, view: HelpOverlayView<'_>, buffer: &mut Buffer) {
    let Some(layout) = help_overlay_layout(area) else {
        return;
    };
    let theme = theme_palette(view.theme);
    fill_rect(layout.frame, dialog_panel_style(&theme), buffer);
    draw_dialog_border(layout.frame, HELP_OVERLAY_TITLE, view.caps, &theme, buffer);
    let row_style = Style::default().fg(theme.text).bg(theme.panel);
    for (visible_row, text) in view
        .rows
        .iter()
        .skip(view.scroll)
        .take(usize::from(layout.rows.height))
        .enumerate()
    {
        let offset = u16::try_from(visible_row).unwrap_or(0);
        let row = Rect::new(
            layout.rows.x,
            layout.rows.y.saturating_add(offset),
            layout.rows.width,
            layout.rows.height.saturating_sub(offset),
        );
        draw_text_row(row, text, row_style, buffer);
    }
    render_help_overlay_more_indicator(layout, view, &theme, buffer);
}

fn render_help_overlay_more_indicator(
    layout: HelpOverlayLayout,
    view: HelpOverlayView<'_>,
    theme: &crate::theme::Theme,
    buffer: &mut Buffer,
) {
    let visible_row_count = usize::from(layout.rows.height).min(view.rows.len());
    let shown_row_count = visible_row_count.min(view.rows.len().saturating_sub(view.scroll));
    if visible_row_count == 0 || view.scroll.saturating_add(shown_row_count) >= view.rows.len() {
        return;
    }
    let label = match view.caps {
        CapsProfile::Basic => HELP_OVERLAY_MORE_ASCII_LABEL,
        CapsProfile::Rich | CapsProfile::Standard => HELP_OVERLAY_MORE_LABEL,
    };
    let label_width = text_display_width(label);
    let column = layout
        .rows
        .right()
        .saturating_sub(label_width)
        .max(layout.rows.x);
    let row = layout.rows.bottom().saturating_sub(1);
    buffer.set_stringn(
        column,
        row,
        label,
        usize::from(label_width),
        Style::default().fg(theme.accent).bg(theme.panel),
    );
}

#[cfg(test)]
mod help_overlay_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    use super::{
        help_overlay_layout, help_overlay_visible_row_count, render_help_overlay, wrap_help_rows,
        HelpOverlayView, HELP_OVERLAY_MAX_HEIGHT, HELP_OVERLAY_MAX_WIDTH, HELP_OVERLAY_TITLE,
    };

    const BINDINGS: [(&str, &str); 3] = [
        (
            "tab, shift+tab",
            "move the focus through Changes, Log and Details",
        ),
        ("ctrl+k", "focus the Changes pane"),
        ("f5", "refresh the repository"),
    ];

    fn row_text(buffer: &Buffer, row: Rect) -> String {
        (row.x..row.right())
            .filter_map(|column| buffer.cell((column, row.y)).map(|cell| cell.symbol()))
            .collect()
    }

    fn rows_text(buffer: &Buffer, rows: Rect) -> String {
        (rows.y..rows.bottom())
            .map(|row| {
                row_text(buffer, Rect::new(rows.x, row, rows.width, 1))
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    #[test]
    fn the_overlay_centres_itself_inside_eighty_by_twenty_four() {
        let layout = help_overlay_layout(Rect::new(0, 0, 80, 24)).expect("the overlay fits");
        assert_eq!(layout.frame.width, HELP_OVERLAY_MAX_WIDTH);
        assert_eq!(layout.frame.height, HELP_OVERLAY_MAX_HEIGHT);
        assert!(layout.frame.right() <= 80);
        assert!(layout.frame.bottom() <= 24);
        assert!(layout.rows.height >= 10);
    }

    #[test]
    fn the_rows_wrap_inside_the_content_width() {
        let layout = help_overlay_layout(Rect::new(0, 0, 80, 24)).expect("the overlay fits");
        let narrow_width = layout.rows.width / 2;
        let rows = wrap_help_rows(&BINDINGS, narrow_width);
        assert!(
            rows.len() > BINDINGS.len(),
            "the long action wraps once the column is narrow"
        );
        for row in &rows {
            assert!(
                crate::shell::text_display_width(row) <= narrow_width,
                "row {row:?} stays inside the content width"
            );
        }
        let wide_rows = wrap_help_rows(&BINDINGS, layout.rows.width);
        assert!(wide_rows[0].starts_with("tab, shift+tab"));
    }

    #[test]
    fn the_overlay_draws_its_title_and_rows() {
        let area = Rect::new(0, 0, 80, 24);
        let layout = help_overlay_layout(area).expect("the overlay fits");
        let rows = wrap_help_rows(&BINDINGS, layout.rows.width);
        let mut buffer = Buffer::empty(area);
        render_help_overlay(
            area,
            HelpOverlayView {
                rows: &rows,
                scroll: 0,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        assert!(row_text(
            &buffer,
            Rect::new(layout.frame.x, layout.frame.y, layout.frame.width, 1)
        )
        .contains(HELP_OVERLAY_TITLE));
        assert!(rows_text(&buffer, layout.rows).contains("ctrl+k"));
    }

    #[test]
    fn scrolling_moves_the_first_visible_row() {
        let area = Rect::new(0, 0, 80, 24);
        let layout = help_overlay_layout(area).expect("the overlay fits");
        let rows = wrap_help_rows(&BINDINGS, layout.rows.width);
        let mut buffer = Buffer::empty(area);
        render_help_overlay(
            area,
            HelpOverlayView {
                rows: &rows,
                scroll: 1,
                caps: CapsProfile::Basic,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        assert_eq!(
            row_text(&buffer, layout.rows).trim_end(),
            rows[1],
            "the scrolled view starts at the second row"
        );
    }

    #[test]
    fn the_visible_row_count_follows_the_screen_height() {
        assert_eq!(
            help_overlay_visible_row_count(Rect::new(0, 0, 80, 24)),
            usize::from(HELP_OVERLAY_MAX_HEIGHT) - 2
        );
        assert_eq!(help_overlay_visible_row_count(Rect::new(0, 0, 10, 10)), 0);
    }

    #[test]
    fn the_basic_profile_keeps_the_overlay_inside_ascii() {
        let area = Rect::new(0, 0, 80, 24);
        let rows = wrap_help_rows(&BINDINGS, 60);
        let mut buffer = Buffer::empty(area);
        render_help_overlay(
            area,
            HelpOverlayView {
                rows: &rows,
                scroll: 0,
                caps: CapsProfile::Basic,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        for column in 0..80 {
            for row in 0..24 {
                let symbol = buffer
                    .cell((column, row))
                    .map(|cell| cell.symbol().to_string())
                    .unwrap_or_default();
                assert!(symbol.is_ascii(), "cell ({column}, {row}) holds {symbol:?}");
            }
        }
    }
}
