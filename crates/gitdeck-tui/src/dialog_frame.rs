use gitdeck_caps::CapsProfile;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::changes::box_glyphs;
use crate::shell::text_display_width;
use crate::theme::Theme;

pub const ORIGIN_REMOTE_NAME: &str = "origin";
pub const DIALOG_SCREEN_MARGIN: u16 = 4;
pub const DIALOG_MIN_WIDTH: u16 = 24;
pub const DIALOG_CONTENT_INSET: u16 = 2;
pub const DIALOG_BUTTON_GAP: u16 = 2;

pub fn centered_dialog_frame(area: Rect, max_width: u16, height: u16) -> Option<Rect> {
    let width = max_width.min(area.width.saturating_sub(DIALOG_SCREEN_MARGIN));
    if width < DIALOG_MIN_WIDTH || area.height < height {
        return None;
    }
    Some(Rect::new(
        area.x.saturating_add(area.width.saturating_sub(width) / 2),
        area.y
            .saturating_add(area.height.saturating_sub(height) / 2),
        width,
        height,
    ))
}

pub fn dialog_content_columns(frame: Rect) -> (u16, u16) {
    (
        frame.x.saturating_add(DIALOG_CONTENT_INSET),
        frame.width.saturating_sub(DIALOG_CONTENT_INSET * 2),
    )
}

pub fn dialog_content_row(frame: Rect, offset: u16) -> Rect {
    let (content_x, content_width) = dialog_content_columns(frame);
    Rect::new(content_x, frame.y.saturating_add(offset), content_width, 1)
}

pub fn label_rect(x: u16, y: u16, label: &str, right: u16) -> Rect {
    let width = text_display_width(label).min(right.saturating_sub(x));
    Rect::new(x, y, width, 1)
}

pub fn dialog_panel_style(theme: &Theme) -> Style {
    Style::default().fg(theme.text).bg(theme.panel)
}

pub fn draw_dialog_border(
    frame: Rect,
    title: &str,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let glyphs = box_glyphs(caps);
    let border_style = Style::default().fg(theme.border).bg(theme.panel);
    let bottom_row = frame.bottom().saturating_sub(1);
    let right_column = frame.right().saturating_sub(1);
    for column in frame.x..frame.right() {
        put_char(buffer, column, frame.y, glyphs.horizontal, border_style);
        put_char(buffer, column, bottom_row, glyphs.horizontal, border_style);
    }
    for row in frame.y..frame.bottom() {
        put_char(buffer, frame.x, row, glyphs.vertical, border_style);
        put_char(buffer, right_column, row, glyphs.vertical, border_style);
    }
    put_char(buffer, frame.x, frame.y, glyphs.top_left, border_style);
    put_char(
        buffer,
        right_column,
        frame.y,
        glyphs.top_right,
        border_style,
    );
    put_char(
        buffer,
        frame.x,
        bottom_row,
        glyphs.bottom_left,
        border_style,
    );
    put_char(
        buffer,
        right_column,
        bottom_row,
        glyphs.bottom_right,
        border_style,
    );
    draw_text_row(
        Rect::new(
            frame.x.saturating_add(1),
            frame.y,
            right_column.saturating_sub(frame.x).saturating_sub(1),
            1,
        ),
        &format!(" {title} "),
        dialog_panel_style(theme),
        buffer,
    );
}

pub fn draw_option_row(
    row: Rect,
    marker: &str,
    checked: bool,
    label: &str,
    focused: bool,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    if row.width == 0 {
        return;
    }
    let marker_color = if checked { theme.text } else { theme.muted };
    let column = draw_text_row(
        row,
        marker,
        Style::default().fg(marker_color).bg(theme.panel),
        buffer,
    );
    draw_text_row(
        Rect::new(
            column.saturating_add(1),
            row.y,
            row.right().saturating_sub(column.saturating_add(1)),
            1,
        ),
        label,
        option_label_style(focused, theme),
        buffer,
    );
}

pub fn option_label_style(focused: bool, theme: &Theme) -> Style {
    if focused {
        return Style::default()
            .fg(theme.accent)
            .bg(theme.panel)
            .add_modifier(Modifier::UNDERLINED);
    }
    Style::default().fg(theme.text).bg(theme.panel)
}

pub fn draw_dialog_button(
    rect: Rect,
    label: &str,
    color: ratatui::style::Color,
    selected: bool,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    if rect.width == 0 {
        return;
    }
    draw_text_row(
        rect,
        label,
        button_label_style(color, selected, theme),
        buffer,
    );
}

pub fn button_label_style(color: ratatui::style::Color, selected: bool, theme: &Theme) -> Style {
    if selected {
        return Style::default()
            .fg(theme.background)
            .bg(theme.accent)
            .add_modifier(Modifier::UNDERLINED);
    }
    Style::default().fg(color).bg(theme.panel)
}

pub fn draw_text_row(row: Rect, text: &str, style: Style, buffer: &mut Buffer) -> u16 {
    if row.width == 0 || row.height == 0 {
        return row.x;
    }
    buffer.set_stringn(row.x, row.y, text, usize::from(row.width), style);
    row.x
        .saturating_add(text_display_width(text).min(row.width))
}

pub fn fill_rect(rect: Rect, style: Style, buffer: &mut Buffer) {
    for row in rect.y..rect.bottom() {
        for column in rect.x..rect.right() {
            if let Some(cell) = buffer.cell_mut((column, row)) {
                cell.set_symbol(" ");
                cell.set_style(style);
            }
        }
    }
}

fn put_char(buffer: &mut Buffer, column: u16, row: u16, character: char, style: Style) {
    if let Some(cell) = buffer.cell_mut((column, row)) {
        cell.set_symbol(&character.to_string());
        cell.set_style(style);
    }
}
