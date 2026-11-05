use std::borrow::Cow;
use std::collections::BTreeSet;
use std::ops::Range;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use gitdeck_caps::CapsProfile;

use crate::display_text::{glyph_display_width, sanitize_display_text, DisplayGlyph};
use crate::theme::Theme;

pub const DIFF_LINE_NUMBER_WIDTH: u16 = 4;
pub const DIFF_OLD_NUMBER_COLUMN: u16 = 0;
pub const DIFF_NEW_NUMBER_COLUMN: u16 = 5;
pub const DIFF_MARKER_COLUMN: u16 = 10;
pub const DIFF_TEXT_COLUMN: u16 = 12;
pub const DIFF_HUNK_CHECKBOX_COLUMN: u16 = 0;
pub const DIFF_HUNK_CHECKBOX_WIDTH: u16 = 3;
pub const DIFF_HUNK_TEXT_COLUMN: u16 = DIFF_HUNK_CHECKBOX_COLUMN + DIFF_HUNK_CHECKBOX_WIDTH;
pub const BINARY_FILE_LABEL: &str = "Binary file";
pub const IDENTICAL_CONTENTS_LABEL: &str = "Contents are identical";
const CHECKED_HUNK_LABEL: &str = "[x]";
const UNCHECKED_HUNK_LABEL: &str = "[ ]";

const ADDED_LINE_BACKGROUND: Color = Color::Rgb(0x25, 0x40, 0x2F);
const REMOVED_LINE_BACKGROUND: Color = Color::Rgb(0x3A, 0x2A, 0x2A);
const CLIPPED_LINE_MARKER: char = '>';
pub const CONTINUATION_MARKER_UNICODE: char = '↓';
pub const CONTINUATION_MARKER_ASCII: char = '~';
const ADDED_LINE_MARKER: char = '+';
const REMOVED_LINE_MARKER: char = '-';
const CONTEXT_LINE_MARKER: char = ' ';
const ANCHORED_ROW_HORIZONTAL_OFFSET: u16 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DiffRowKind {
    HunkHeader,
    Context,
    Added,
    Removed,
    NoNewlineMarker,
    Notice,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffRow<'a> {
    pub kind: DiffRowKind,
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
    pub hunk_index: Option<usize>,
    pub highlight: Option<Range<usize>>,
    pub text: Cow<'a, str>,
}

pub struct DiffPaneView<'a> {
    pub rows: &'a [DiffRow<'a>],
    pub scroll: usize,
    pub horizontal_offset: u16,
    pub wrap_width: Option<u16>,
    pub checked_hunks: Option<&'a BTreeSet<usize>>,
    pub caps: CapsProfile,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiffDisplayRow {
    pub logical_row_index: usize,
    pub continuation: bool,
    pub start_column: u16,
}

pub fn diff_display_rows(
    rows: &[DiffRow<'_>],
    wrap_width: Option<u16>,
    caps: CapsProfile,
) -> Vec<DiffDisplayRow> {
    let mut display_rows = Vec::with_capacity(rows.len());
    for (logical_row_index, diff_row) in rows.iter().enumerate() {
        match wrap_width {
            Some(width) if width > 0 && wraps_softly(diff_row.kind) => {
                for (offset, start_column) in row_wrap_columns(diff_row.text.as_ref(), width, caps)
                    .into_iter()
                    .enumerate()
                {
                    display_rows.push(DiffDisplayRow {
                        logical_row_index,
                        continuation: offset > 0,
                        start_column,
                    });
                }
            }
            _ => display_rows.push(DiffDisplayRow {
                logical_row_index,
                continuation: false,
                start_column: 0,
            }),
        }
    }
    display_rows
}

fn wraps_softly(row_kind: DiffRowKind) -> bool {
    matches!(
        row_kind,
        DiffRowKind::Context | DiffRowKind::Added | DiffRowKind::Removed
    )
}

fn row_wrap_columns(text: &str, wrap_width: u16, caps: CapsProfile) -> Vec<u16> {
    let sanitized = sanitize_display_text(text, DIFF_TEXT_COLUMN, caps);
    let mut starts = vec![0u16];
    let mut row_column = 0u16;
    let mut display_column = 0u16;
    for glyph in sanitized.glyphs() {
        let width = glyph_display_width(glyph.character);
        if row_column > 0 && row_column.saturating_add(width) > wrap_width {
            starts.push(display_column);
            row_column = 0;
        }
        row_column = row_column.saturating_add(width);
        display_column = display_column.saturating_add(width);
    }
    starts
}

fn continuation_marker(caps: CapsProfile) -> char {
    match caps {
        CapsProfile::Rich | CapsProfile::Standard => CONTINUATION_MARKER_UNICODE,
        CapsProfile::Basic => CONTINUATION_MARKER_ASCII,
    }
}

pub fn render_diff_pane(area: Rect, view: DiffPaneView<'_>, theme: &Theme, buffer: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    fill_area(area, row_style(DiffRowKind::Context, theme), buffer);
    let wraps = view.wrap_width.is_some_and(|width| width > 0);
    let display_rows = diff_display_rows(view.rows, view.wrap_width, view.caps);
    let visible_rows = display_rows
        .iter()
        .skip(view.scroll)
        .take(usize::from(area.height));
    for (offset, display_row) in visible_rows.enumerate() {
        let Some(row_y) = u16::try_from(offset)
            .ok()
            .and_then(|offset| area.y.checked_add(offset))
        else {
            break;
        };
        let Some(diff_row) = view.rows.get(display_row.logical_row_index) else {
            break;
        };
        let wrapped = wraps && wraps_softly(diff_row.kind);
        let placement = DisplayRowPlacement {
            continuation: wrapped && display_row.continuation,
            start_column: if wrapped {
                display_row.start_column
            } else {
                view.horizontal_offset
            },
            clip_text: !wrapped,
        };
        render_diff_row(
            area,
            row_y,
            diff_row,
            placement,
            RowPaint {
                theme,
                checked_hunks: view.checked_hunks,
                caps: view.caps,
            },
            buffer,
        );
    }
}

fn fill_area(area: Rect, style: Style, buffer: &mut Buffer) {
    for row_y in area.y..area.bottom() {
        for column in area.x..area.right() {
            put_glyph(buffer, column, row_y, ' ', 1, style);
        }
    }
}

struct RowPaint<'a> {
    theme: &'a Theme,
    checked_hunks: Option<&'a BTreeSet<usize>>,
    caps: CapsProfile,
}

struct DisplayRowPlacement {
    continuation: bool,
    start_column: u16,
    clip_text: bool,
}

struct LineTextRequest<'a> {
    column: u16,
    text: &'a str,
    style: Style,
    highlight: Option<(&'a Range<usize>, Style)>,
    start_column: u16,
    clip_text: bool,
}

pub fn hunk_checkbox_rect(area: Rect, row_y: u16) -> Rect {
    Rect::new(
        area.x.saturating_add(DIFF_HUNK_CHECKBOX_COLUMN),
        row_y,
        DIFF_HUNK_CHECKBOX_WIDTH,
        1,
    )
}

fn render_diff_row(
    area: Rect,
    row_y: u16,
    diff_row: &DiffRow<'_>,
    placement: DisplayRowPlacement,
    paint: RowPaint<'_>,
    buffer: &mut Buffer,
) {
    let theme = paint.theme;
    let row_rect = Rect::new(area.x, row_y, area.width, 1);
    buffer.set_style(row_rect, row_style(diff_row.kind, theme));
    match diff_row.kind {
        DiffRowKind::HunkHeader => {
            let text_column =
                render_hunk_checkbox(area, row_rect, diff_row, theme, buffer, paint.checked_hunks);
            draw_line_text(
                buffer,
                row_rect,
                paint.caps,
                LineTextRequest {
                    column: text_column,
                    text: &diff_row.text,
                    style: muted_text_style(diff_row.kind, theme),
                    highlight: None,
                    start_column: ANCHORED_ROW_HORIZONTAL_OFFSET,
                    clip_text: true,
                },
            );
        }
        DiffRowKind::NoNewlineMarker | DiffRowKind::Notice => {
            draw_line_text(
                buffer,
                row_rect,
                paint.caps,
                LineTextRequest {
                    column: DIFF_TEXT_COLUMN,
                    text: &diff_row.text,
                    style: muted_text_style(diff_row.kind, theme),
                    highlight: None,
                    start_column: placement.start_column,
                    clip_text: placement.clip_text,
                },
            );
        }
        DiffRowKind::Context | DiffRowKind::Added | DiffRowKind::Removed => {
            if !placement.continuation {
                render_line_numbers(buffer, row_rect, diff_row, theme);
            }
            let marker = if placement.continuation {
                continuation_marker(paint.caps)
            } else {
                line_marker(diff_row.kind)
            };
            put_glyph(
                buffer,
                row_rect.x.saturating_add(DIFF_MARKER_COLUMN),
                row_y,
                marker,
                1,
                marker_style(diff_row.kind, theme),
            );
            draw_line_text(
                buffer,
                row_rect,
                paint.caps,
                LineTextRequest {
                    column: DIFF_TEXT_COLUMN,
                    text: &diff_row.text,
                    style: line_text_style(diff_row.kind, theme),
                    highlight: diff_row
                        .highlight
                        .as_ref()
                        .map(|range| (range, word_highlight_style(diff_row.kind, theme))),
                    start_column: placement.start_column,
                    clip_text: placement.clip_text,
                },
            );
        }
    }
}

fn render_hunk_checkbox(
    area: Rect,
    row_rect: Rect,
    diff_row: &DiffRow<'_>,
    theme: &Theme,
    buffer: &mut Buffer,
    checked_hunks: Option<&BTreeSet<usize>>,
) -> u16 {
    let Some(checked_hunks) = checked_hunks else {
        return 0;
    };
    let Some(hunk_index) = diff_row.hunk_index else {
        return 0;
    };
    let checked = checked_hunks.contains(&hunk_index);
    let foreground = if checked { theme.accent } else { theme.muted };
    let label = if checked {
        CHECKED_HUNK_LABEL
    } else {
        UNCHECKED_HUNK_LABEL
    };
    let checkbox = hunk_checkbox_rect(area, row_rect.y);
    draw_glyphs(
        buffer,
        checkbox.x,
        checkbox.y,
        checkbox.width,
        label,
        Style::default()
            .fg(foreground)
            .bg(row_background(diff_row.kind, theme)),
    );
    DIFF_HUNK_TEXT_COLUMN
}

fn render_line_numbers(buffer: &mut Buffer, row_rect: Rect, diff_row: &DiffRow<'_>, theme: &Theme) {
    for (column, line_number) in [
        (DIFF_OLD_NUMBER_COLUMN, diff_row.old_line_number),
        (DIFF_NEW_NUMBER_COLUMN, diff_row.new_line_number),
    ] {
        let label = line_number_label(line_number);
        if label.is_empty() {
            continue;
        }
        draw_column_text(
            buffer,
            row_rect,
            column,
            DIFF_LINE_NUMBER_WIDTH,
            &label,
            muted_text_style(diff_row.kind, theme),
        );
    }
}

fn line_number_label(line_number: Option<u32>) -> String {
    let Some(line_number) = line_number else {
        return String::new();
    };
    let digits = line_number.to_string();
    let padding = usize::from(DIFF_LINE_NUMBER_WIDTH).saturating_sub(digits.len());
    format!("{}{digits}", " ".repeat(padding))
}

fn line_marker(row_kind: DiffRowKind) -> char {
    match row_kind {
        DiffRowKind::Added => ADDED_LINE_MARKER,
        DiffRowKind::Removed => REMOVED_LINE_MARKER,
        DiffRowKind::Context
        | DiffRowKind::HunkHeader
        | DiffRowKind::NoNewlineMarker
        | DiffRowKind::Notice => CONTEXT_LINE_MARKER,
    }
}

fn draw_line_text(
    buffer: &mut Buffer,
    row_rect: Rect,
    caps: CapsProfile,
    request: LineTextRequest<'_>,
) {
    let text_column = row_rect.x.saturating_add(request.column);
    let available_width = row_rect.right().saturating_sub(text_column);
    let sanitized = sanitize_display_text(request.text, request.column, caps);
    let first_visible_glyph = sanitized.glyph_index_at_display_column(request.start_column);
    let visible_glyphs = &sanitized.glyphs()[first_visible_glyph..];
    let paint = GlyphPaint {
        style: request.style,
        highlight: request.highlight,
    };
    let needs_marker = request.clip_text
        && (request.start_column > 0 || sanitized.display_width() > available_width);
    if needs_marker {
        paint_glyphs(
            buffer,
            text_column,
            row_rect.y,
            available_width.saturating_sub(1),
            visible_glyphs,
            paint,
        );
        put_glyph(
            buffer,
            row_rect.right().saturating_sub(1),
            row_rect.y,
            CLIPPED_LINE_MARKER,
            1,
            request.style,
        );
        return;
    }
    paint_glyphs(
        buffer,
        text_column,
        row_rect.y,
        available_width,
        visible_glyphs,
        paint,
    );
}

fn draw_column_text(
    buffer: &mut Buffer,
    row_rect: Rect,
    column: u16,
    max_width: u16,
    text: &str,
    style: Style,
) {
    let text_column = row_rect.x.saturating_add(column);
    let available_width = row_rect.right().saturating_sub(text_column).min(max_width);
    draw_glyphs(
        buffer,
        text_column,
        row_rect.y,
        available_width,
        text,
        style,
    );
}

fn draw_glyphs(buffer: &mut Buffer, x: u16, y: u16, max_width: u16, text: &str, style: Style) {
    let limit = x.saturating_add(max_width);
    let mut column = x;
    for character in text.chars() {
        let width = glyph_display_width(character);
        if column.saturating_add(width) > limit {
            return;
        }
        put_glyph(buffer, column, y, character, width, style);
        column = column.saturating_add(width);
    }
}

struct GlyphPaint<'a> {
    style: Style,
    highlight: Option<(&'a Range<usize>, Style)>,
}

fn paint_glyphs(
    buffer: &mut Buffer,
    x: u16,
    y: u16,
    max_width: u16,
    glyphs: &[DisplayGlyph],
    paint: GlyphPaint<'_>,
) {
    let limit = x.saturating_add(max_width);
    let mut column = x;
    for glyph in glyphs {
        let width = glyph_display_width(glyph.character);
        if column.saturating_add(width) > limit {
            return;
        }
        let glyph_style = match paint.highlight {
            Some((range, highlight_style)) if range.contains(&glyph.source_index) => {
                highlight_style
            }
            _ => paint.style,
        };
        put_glyph(buffer, column, y, glyph.character, width, glyph_style);
        column = column.saturating_add(width);
    }
}

fn put_glyph(buffer: &mut Buffer, x: u16, y: u16, symbol: char, width: u16, style: Style) {
    let mut encoded = [0u8; 4];
    let encoded_symbol = symbol.encode_utf8(&mut encoded);
    if let Some(cell) = buffer.cell_mut((x, y)) {
        cell.set_symbol(encoded_symbol);
        cell.set_style(style);
    }
    for offset in 1..width {
        if let Some(cell) = buffer.cell_mut((x.saturating_add(offset), y)) {
            cell.set_symbol(" ");
            cell.set_style(style);
        }
    }
}

fn row_background(row_kind: DiffRowKind, theme: &Theme) -> Color {
    match row_kind {
        DiffRowKind::HunkHeader => theme.panel,
        DiffRowKind::Added => ADDED_LINE_BACKGROUND,
        DiffRowKind::Removed => REMOVED_LINE_BACKGROUND,
        DiffRowKind::Context | DiffRowKind::NoNewlineMarker | DiffRowKind::Notice => {
            theme.background
        }
    }
}

fn row_style(row_kind: DiffRowKind, theme: &Theme) -> Style {
    Style::default()
        .fg(theme.text)
        .bg(row_background(row_kind, theme))
}

fn muted_text_style(row_kind: DiffRowKind, theme: &Theme) -> Style {
    Style::default()
        .fg(theme.muted)
        .bg(row_background(row_kind, theme))
}

fn line_text_style(row_kind: DiffRowKind, theme: &Theme) -> Style {
    Style::default()
        .fg(theme.text)
        .bg(row_background(row_kind, theme))
}

fn word_highlight_style(row_kind: DiffRowKind, theme: &Theme) -> Style {
    let background = match row_kind {
        DiffRowKind::Added => theme.added_word_highlight,
        DiffRowKind::Removed => theme.removed_word_highlight,
        DiffRowKind::Context
        | DiffRowKind::HunkHeader
        | DiffRowKind::NoNewlineMarker
        | DiffRowKind::Notice => row_background(row_kind, theme),
    };
    Style::default().fg(theme.text).bg(background)
}

fn marker_style(row_kind: DiffRowKind, theme: &Theme) -> Style {
    let foreground = match row_kind {
        DiffRowKind::Added => theme.success,
        DiffRowKind::Removed => theme.error,
        DiffRowKind::Context
        | DiffRowKind::HunkHeader
        | DiffRowKind::NoNewlineMarker
        | DiffRowKind::Notice => theme.text,
    };
    Style::default()
        .fg(foreground)
        .bg(row_background(row_kind, theme))
}

#[cfg(test)]
mod diff_pane_tests {
    use std::borrow::Cow;
    use std::collections::BTreeSet;

    use gitdeck_caps::{CapsProfile, ThemeId};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    use crate::display_text::glyph_display_width;

    use super::{
        diff_display_rows, render_diff_pane, DiffPaneView, DiffRow, DiffRowKind,
        ADDED_LINE_BACKGROUND, BINARY_FILE_LABEL, CONTINUATION_MARKER_ASCII,
        CONTINUATION_MARKER_UNICODE, DIFF_HUNK_TEXT_COLUMN, DIFF_MARKER_COLUMN,
        DIFF_NEW_NUMBER_COLUMN, DIFF_OLD_NUMBER_COLUMN, DIFF_TEXT_COLUMN, REMOVED_LINE_BACKGROUND,
    };
    use crate::theme::theme_palette;
    use crate::word_highlight::assign_word_highlights;

    const TEXT_COLOR: Color = Color::Rgb(0xDF, 0xE1, 0xE5);
    const MUTED_COLOR: Color = Color::Rgb(0x86, 0x8A, 0x91);
    const PANEL_COLOR: Color = Color::Rgb(0x2B, 0x2D, 0x30);
    const SUCCESS_COLOR: Color = Color::Rgb(0x57, 0x96, 0x5C);
    const ERROR_COLOR: Color = Color::Rgb(0xDB, 0x5C, 0x5C);
    const ACCENT_COLOR: Color = Color::Rgb(0x35, 0x74, 0xF0);
    const ADDED_WORD_HIGHLIGHT: Color = Color::Rgb(0x3A, 0x6B, 0x47);
    const REMOVED_WORD_HIGHLIGHT: Color = Color::Rgb(0x6B, 0x3A, 0x3A);

    fn row(
        kind: DiffRowKind,
        old_line_number: Option<u32>,
        new_line_number: Option<u32>,
        text: &str,
    ) -> DiffRow<'_> {
        DiffRow {
            kind,
            old_line_number,
            new_line_number,
            hunk_index: None,
            highlight: None,
            text: Cow::Borrowed(text),
        }
    }

    fn hunk_header_row(hunk_index: usize, text: &str) -> DiffRow<'_> {
        DiffRow {
            kind: DiffRowKind::HunkHeader,
            old_line_number: None,
            new_line_number: None,
            hunk_index: Some(hunk_index),
            highlight: None,
            text: Cow::Borrowed(text),
        }
    }

    fn render(rows: &[DiffRow<'_>], scroll: usize, width: u16, height: u16) -> Buffer {
        render_with_offset(rows, scroll, width, height, 0)
    }

    fn render_with_offset(
        rows: &[DiffRow<'_>],
        scroll: usize,
        width: u16,
        height: u16,
        horizontal_offset: u16,
    ) -> Buffer {
        render_with_checked_hunks(rows, scroll, width, height, horizontal_offset, None)
    }

    fn render_with_caps(
        rows: &[DiffRow<'_>],
        caps: CapsProfile,
        width: u16,
        height: u16,
    ) -> Buffer {
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
        render_diff_pane(
            Rect::new(0, 0, width, height),
            DiffPaneView {
                rows,
                scroll: 0,
                horizontal_offset: 0,
                wrap_width: None,
                checked_hunks: None,
                caps,
            },
            &theme_palette(ThemeId::JbDark),
            &mut buffer,
        );
        buffer
    }

    fn render_with_checked_hunks(
        rows: &[DiffRow<'_>],
        scroll: usize,
        width: u16,
        height: u16,
        horizontal_offset: u16,
        checked_hunks: Option<&BTreeSet<usize>>,
    ) -> Buffer {
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
        render_diff_pane(
            Rect::new(0, 0, width, height),
            DiffPaneView {
                rows,
                scroll,
                horizontal_offset,
                wrap_width: None,
                checked_hunks,
                caps: CapsProfile::Rich,
            },
            &theme_palette(ThemeId::JbDark),
            &mut buffer,
        );
        buffer
    }

    fn render_wrapped(
        rows: &[DiffRow<'_>],
        wrap_width: u16,
        width: u16,
        height: u16,
        caps: CapsProfile,
    ) -> Buffer {
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
        render_diff_pane(
            Rect::new(0, 0, width, height),
            DiffPaneView {
                rows,
                scroll: 0,
                horizontal_offset: 0,
                wrap_width: Some(wrap_width),
                checked_hunks: None,
                caps,
            },
            &theme_palette(ThemeId::JbDark),
            &mut buffer,
        );
        buffer
    }

    fn line_text(buffer: &Buffer, row_index: u16) -> String {
        let mut line = String::new();
        for column in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((column, row_index)) {
                line.push_str(cell.symbol());
            }
        }
        line.trim_end().to_string()
    }

    fn symbol(buffer: &Buffer, column: u16, row_index: u16) -> String {
        buffer
            .cell((column, row_index))
            .map(|cell| cell.symbol().to_string())
            .unwrap_or_default()
    }

    #[test]
    fn context_line_numbers_sit_in_the_frozen_columns() {
        let rows = [row(DiffRowKind::Context, Some(7), Some(9), "keep")];
        let buffer = render(&rows, 0, 24, 1);
        assert_eq!(line_text(&buffer, 0), "   7    9   keep");
        assert_eq!(symbol(&buffer, DIFF_OLD_NUMBER_COLUMN + 3, 0), "7");
        assert_eq!(symbol(&buffer, DIFF_NEW_NUMBER_COLUMN + 3, 0), "9");
        assert_eq!(symbol(&buffer, DIFF_MARKER_COLUMN, 0), " ");
        assert_eq!(symbol(&buffer, DIFF_TEXT_COLUMN, 0), "k");
    }

    #[test]
    fn added_and_removed_rows_paint_their_own_backgrounds() {
        let rows = [
            row(DiffRowKind::Added, None, Some(4), "added"),
            row(DiffRowKind::Removed, Some(4), None, "removed"),
        ];
        let buffer = render(&rows, 0, 24, 2);
        for column in 0..24 {
            let added_cell = buffer.cell((column, 0)).expect("added cell");
            let removed_cell = buffer.cell((column, 1)).expect("removed cell");
            assert_eq!(added_cell.bg, ADDED_LINE_BACKGROUND, "added at {column}");
            assert_eq!(
                removed_cell.bg, REMOVED_LINE_BACKGROUND,
                "removed at {column}"
            );
        }
        assert_eq!(symbol(&buffer, DIFF_MARKER_COLUMN, 0), "+");
        assert_eq!(symbol(&buffer, DIFF_MARKER_COLUMN, 1), "-");
        assert_eq!(
            buffer.cell((DIFF_MARKER_COLUMN, 0)).expect("cell").fg,
            SUCCESS_COLOR
        );
        assert_eq!(
            buffer.cell((DIFF_MARKER_COLUMN, 1)).expect("cell").fg,
            ERROR_COLOR
        );
        assert_eq!(
            buffer.cell((DIFF_TEXT_COLUMN, 0)).expect("cell").fg,
            TEXT_COLOR
        );
    }

    #[test]
    fn hunk_header_starts_at_column_zero_with_the_panel_background() {
        let rows = [row(DiffRowKind::HunkHeader, None, None, "@@ -1,3 +1,3 @@")];
        let buffer = render(&rows, 0, 24, 1);
        assert_eq!(line_text(&buffer, 0), "@@ -1,3 +1,3 @@");
        assert_eq!(buffer.cell((0, 0)).expect("cell").bg, PANEL_COLOR);
        assert_eq!(buffer.cell((0, 0)).expect("cell").fg, MUTED_COLOR);
    }

    #[test]
    fn a_hunk_header_without_hunk_staging_keeps_its_row_start() {
        let rows = [hunk_header_row(0, "@@ -1,3 +1,3 @@")];
        let buffer = render(&rows, 0, 24, 1);
        assert_eq!(line_text(&buffer, 0), "@@ -1,3 +1,3 @@");
        assert_eq!(symbol(&buffer, 0, 0), "@");
    }

    #[test]
    fn checked_hunks_paint_an_accent_checkbox_before_the_header_text() {
        let rows = [hunk_header_row(0, "@@ -1,3 +1,3 @@")];
        let mut checked = BTreeSet::new();
        checked.insert(0usize);
        let buffer = render_with_checked_hunks(&rows, 0, 24, 1, 0, Some(&checked));
        assert_eq!(line_text(&buffer, 0), "[x]@@ -1,3 +1,3 @@");
        assert_eq!(buffer.cell((0, 0)).expect("cell").fg, ACCENT_COLOR);
        assert_eq!(buffer.cell((3, 0)).expect("cell").fg, MUTED_COLOR);
    }

    #[test]
    fn unchecked_hunks_paint_a_muted_empty_checkbox() {
        let rows = [hunk_header_row(0, "@@ -1,3 +1,3 @@")];
        let checked = BTreeSet::new();
        let buffer = render_with_checked_hunks(&rows, 0, 24, 1, 0, Some(&checked));
        assert_eq!(line_text(&buffer, 0), "[ ]@@ -1,3 +1,3 @@");
        assert_eq!(buffer.cell((0, 0)).expect("cell").fg, MUTED_COLOR);
        assert_eq!(symbol(&buffer, DIFF_HUNK_TEXT_COLUMN, 0), "@");
    }

    #[test]
    fn notice_rows_start_at_the_text_column() {
        let rows = [row(DiffRowKind::Notice, None, None, BINARY_FILE_LABEL)];
        let buffer = render(&rows, 0, 24, 1);
        assert_eq!(line_text(&buffer, 0), "            Binary file");
    }

    #[test]
    fn missing_newline_marker_starts_at_the_text_column() {
        let rows = [row(
            DiffRowKind::NoNewlineMarker,
            None,
            None,
            "\\ No newline at end of file",
        )];
        let buffer = render(&rows, 0, 40, 1);
        assert_eq!(
            line_text(&buffer, 0),
            "            \\ No newline at end of file"
        );
    }

    #[test]
    fn long_text_is_clipped_and_marked_at_the_last_column() {
        let long_text = "x".repeat(40);
        let rows = [row(DiffRowKind::Context, Some(1), Some(1), &long_text)];
        let buffer = render(&rows, 0, 24, 1);
        assert_eq!(symbol(&buffer, 23, 0), ">");
        assert_eq!(symbol(&buffer, 22, 0), "x");
        assert_eq!(line_text(&buffer, 0).chars().count(), 24);
    }

    #[test]
    fn horizontal_offset_skips_the_leading_columns_and_marks_the_row_end() {
        let rows = [row(DiffRowKind::Context, Some(1), Some(1), "abcdefghij")];
        let buffer = render_with_offset(&rows, 0, 24, 1, 4);
        assert_eq!(line_text(&buffer, 0), "   1    1   efghij     >");
        assert_eq!(symbol(&buffer, DIFF_TEXT_COLUMN, 0), "e");
        assert_eq!(symbol(&buffer, 23, 0), ">");
    }

    #[test]
    fn a_row_without_a_horizontal_offset_drops_the_marker_when_it_fits() {
        let rows = [row(DiffRowKind::Context, Some(1), Some(1), "abcdefghij")];
        let buffer = render(&rows, 0, 24, 1);
        assert_eq!(line_text(&buffer, 0), "   1    1   abcdefghij");
        assert_eq!(symbol(&buffer, 23, 0), " ");
    }

    #[test]
    fn the_hunk_header_stays_anchored_when_the_body_scrolls_sideways() {
        let rows = [
            row(DiffRowKind::HunkHeader, None, None, "@@ -2,14 +2,14 @@"),
            row(DiffRowKind::Context, Some(2), Some(2), "abcdefghij"),
        ];
        let buffer = render_with_offset(&rows, 0, 24, 2, 4);
        assert_eq!(line_text(&buffer, 0), "@@ -2,14 +2,14 @@");
        assert_eq!(symbol(&buffer, 0, 0), "@");
        assert_eq!(symbol(&buffer, DIFF_TEXT_COLUMN, 1), "e");
    }

    #[test]
    fn scrolling_skips_the_leading_rows() {
        let rows = [
            row(DiffRowKind::Context, Some(1), Some(1), "first"),
            row(DiffRowKind::Context, Some(2), Some(2), "second"),
        ];
        let buffer = render(&rows, 1, 24, 1);
        assert_eq!(line_text(&buffer, 0), "   2    2   second");
        let scrolled_past_the_end = render(&rows, 5, 24, 1);
        assert_eq!(line_text(&scrolled_past_the_end, 0), "");
    }

    fn background(buffer: &Buffer, column: u16, row_index: u16) -> Color {
        buffer
            .cell((column, row_index))
            .map(|cell| cell.bg)
            .unwrap_or(Color::Reset)
    }

    fn highlighted_rows<'a>(removed: &'a [&'a str], added: &'a [&'a str]) -> Vec<DiffRow<'a>> {
        let mut rows: Vec<DiffRow<'_>> = removed
            .iter()
            .enumerate()
            .map(|(index, text)| {
                row(
                    DiffRowKind::Removed,
                    Some(u32::try_from(index).unwrap_or(0) + 1),
                    None,
                    text,
                )
            })
            .collect();
        rows.extend(added.iter().enumerate().map(|(index, text)| {
            row(
                DiffRowKind::Added,
                None,
                Some(u32::try_from(index).unwrap_or(0) + 1),
                text,
            )
        }));
        assign_word_highlights(&mut rows);
        rows
    }

    #[test]
    fn word_highlights_bracket_the_changed_tokens() {
        let rows = highlighted_rows(&["let a = 1;"], &["let b = 2;"]);
        assert_eq!(rows[0].highlight, Some(4..9));
        assert_eq!(rows[1].highlight, Some(4..9));
        let buffer = render(&rows, 0, 40, 2);
        for column in DIFF_TEXT_COLUMN..DIFF_TEXT_COLUMN + 4 {
            assert_eq!(
                background(&buffer, column, 0),
                REMOVED_LINE_BACKGROUND,
                "the shared let prefix at column {column} stays unhighlighted"
            );
            assert_eq!(
                background(&buffer, column, 1),
                ADDED_LINE_BACKGROUND,
                "the shared let prefix at column {column} stays unhighlighted"
            );
        }
        for column in DIFF_TEXT_COLUMN + 4..DIFF_TEXT_COLUMN + 9 {
            assert_eq!(
                background(&buffer, column, 0),
                REMOVED_WORD_HIGHLIGHT,
                "the changed token at column {column} is highlighted"
            );
            assert_eq!(
                background(&buffer, column, 1),
                ADDED_WORD_HIGHLIGHT,
                "the changed token at column {column} is highlighted"
            );
        }
        assert_eq!(
            background(&buffer, DIFF_TEXT_COLUMN + 9, 0),
            REMOVED_LINE_BACKGROUND
        );
        assert_eq!(
            background(&buffer, DIFF_TEXT_COLUMN + 9, 1),
            ADDED_LINE_BACKGROUND
        );
    }

    #[test]
    fn identical_lines_produce_no_highlight() {
        let rows = highlighted_rows(&["let a = 1;"], &["let a = 1;"]);
        assert_eq!(rows[0].highlight, None);
        assert_eq!(rows[1].highlight, None);
        let buffer = render(&rows, 0, 40, 2);
        for column in DIFF_TEXT_COLUMN..DIFF_TEXT_COLUMN + 10 {
            assert_eq!(background(&buffer, column, 0), REMOVED_LINE_BACKGROUND);
            assert_eq!(background(&buffer, column, 1), ADDED_LINE_BACKGROUND);
        }
    }

    #[test]
    fn uneven_removed_and_added_runs_highlight_the_extra_lines_whole() {
        let rows = highlighted_rows(
            &["let a = 1;", "let b = 2;"],
            &["let a = 9;", "let b = 8;", "let c = 3;"],
        );
        assert_eq!(rows[0].highlight, Some(8..9));
        assert_eq!(rows[1].highlight, Some(8..9));
        assert_eq!(rows[2].highlight, Some(8..9));
        assert_eq!(rows[3].highlight, Some(8..9));
        assert_eq!(rows[4].highlight, Some(0..10));
        let buffer = render(&rows, 0, 40, 5);
        for column in DIFF_TEXT_COLUMN..DIFF_TEXT_COLUMN + 10 {
            assert_eq!(
                background(&buffer, column, 4),
                ADDED_WORD_HIGHLIGHT,
                "the unpaired added line is highlighted at column {column}"
            );
        }
        assert_eq!(
            background(&buffer, DIFF_TEXT_COLUMN, 2),
            ADDED_LINE_BACKGROUND,
            "the paired added line keeps the unshared prefix unhighlighted"
        );
    }

    #[test]
    fn a_diff_with_tabs_renders_without_panicking() {
        let rows = [
            row(DiffRowKind::Removed, Some(1), None, "a\tb\n"),
            row(DiffRowKind::Added, None, Some(1), "a\tB"),
        ];
        let buffer = render(&rows, 0, 40, 2);
        for row_index in 0..2 {
            let text = line_text(&buffer, row_index);
            assert!(
                !text.contains(char::is_control),
                "row {row_index} leaked a control character: {text:?}"
            );
        }
        assert_eq!(symbol(&buffer, DIFF_TEXT_COLUMN, 0), "a");
        assert_eq!(symbol(&buffer, DIFF_TEXT_COLUMN + 4, 0), "b");
        assert_eq!(symbol(&buffer, DIFF_TEXT_COLUMN + 5, 0), "·");
        assert_eq!(symbol(&buffer, DIFF_TEXT_COLUMN + 4, 1), "B");
    }

    #[test]
    fn tab_expansion_respects_the_line_number_gutter_width() {
        let rows = [row(DiffRowKind::Context, Some(1), Some(1), "\tx")];
        let buffer = render(&rows, 0, 40, 1);
        let first_space = DIFF_TEXT_COLUMN + 4;
        assert_eq!(
            symbol(&buffer, DIFF_TEXT_COLUMN, 0),
            " ",
            "the tab starts at the diff text column"
        );
        assert_eq!(symbol(&buffer, first_space, 0), "x");
        assert_eq!(symbol(&buffer, first_space - 1, 0), " ");
        let ascii_buffer = render_with_caps(&rows, CapsProfile::Basic, 40, 1);
        assert_eq!(symbol(&ascii_buffer, first_space, 0), "x");
    }

    #[test]
    fn word_highlights_still_align_after_tab_expansion() {
        let rows = highlighted_rows(&["a\tb"], &["a\tB"]);
        assert_eq!(rows[0].highlight, Some(2..3));
        assert_eq!(rows[1].highlight, Some(2..3));
        let buffer = render(&rows, 0, 40, 2);
        let changed_column = DIFF_TEXT_COLUMN + 4;
        assert_eq!(symbol(&buffer, changed_column, 0), "b");
        assert_eq!(
            background(&buffer, changed_column, 0),
            REMOVED_WORD_HIGHLIGHT,
            "the changed glyph sits four columns after the text column"
        );
        assert_eq!(
            background(&buffer, changed_column, 1),
            ADDED_WORD_HIGHLIGHT,
            "the changed glyph sits four columns after the text column"
        );
        for column in DIFF_TEXT_COLUMN + 1..changed_column {
            assert_eq!(
                background(&buffer, column, 0),
                REMOVED_LINE_BACKGROUND,
                "the tab expansion at column {column} stays unhighlighted"
            );
            assert_eq!(background(&buffer, column, 1), ADDED_LINE_BACKGROUND);
        }
    }

    #[test]
    fn cjk_ranges_do_not_split_cells() {
        let rows = highlighted_rows(&["中文行内容"], &["中文段落内容"]);
        assert_eq!(rows[0].highlight, Some(2..3));
        assert_eq!(rows[1].highlight, Some(2..4));
        let buffer = render(&rows, 0, 40, 2);
        for column in DIFF_TEXT_COLUMN..DIFF_TEXT_COLUMN + 4 {
            assert_eq!(
                background(&buffer, column, 0),
                REMOVED_LINE_BACKGROUND,
                "the shared cjk prefix at column {column} stays unhighlighted"
            );
        }
        assert_eq!(
            background(&buffer, DIFF_TEXT_COLUMN + 4, 0),
            REMOVED_WORD_HIGHLIGHT
        );
        assert_eq!(
            background(&buffer, DIFF_TEXT_COLUMN + 5, 0),
            REMOVED_WORD_HIGHLIGHT,
            "the wide glyph keeps both of its cells highlighted"
        );
        assert_eq!(
            background(&buffer, DIFF_TEXT_COLUMN + 6, 0),
            REMOVED_LINE_BACKGROUND,
            "the trailing cjk glyph keeps its own background"
        );
        for column in DIFF_TEXT_COLUMN + 4..DIFF_TEXT_COLUMN + 8 {
            assert_eq!(
                background(&buffer, column, 1),
                ADDED_WORD_HIGHLIGHT,
                "both changed wide glyphs stay whole at column {column}"
            );
        }
        assert_eq!(
            background(&buffer, DIFF_TEXT_COLUMN + 8, 1),
            ADDED_LINE_BACKGROUND
        );
        assert_eq!(symbol(&buffer, DIFF_TEXT_COLUMN + 4, 1), "段");
        assert_eq!(symbol(&buffer, DIFF_TEXT_COLUMN + 6, 1), "落");
    }

    #[test]
    fn highlight_survives_horizontal_offset() {
        let rows = highlighted_rows(
            &["pub const ALPHA: u32 = 7;"],
            &["pub const ALPHA: u32 = 5;"],
        );
        assert_eq!(rows[0].highlight, Some(23..24));
        assert_eq!(rows[1].highlight, Some(23..24));
        let anchored = render(&rows, 0, 48, 1);
        assert_eq!(
            background(&anchored, DIFF_TEXT_COLUMN + 23, 0),
            REMOVED_WORD_HIGHLIGHT
        );
        assert_eq!(
            background(&anchored, DIFF_TEXT_COLUMN + 22, 0),
            REMOVED_LINE_BACKGROUND
        );
        let shifted = render_with_offset(&rows, 0, 48, 1, 12);
        assert_eq!(
            background(&shifted, DIFF_TEXT_COLUMN + 11, 0),
            REMOVED_WORD_HIGHLIGHT,
            "the token sits twelve columns earlier while the pane is offset"
        );
        assert_eq!(
            background(&shifted, DIFF_TEXT_COLUMN + 10, 0),
            REMOVED_LINE_BACKGROUND
        );
        assert_eq!(
            symbol(&shifted, DIFF_TEXT_COLUMN + 11, 0),
            symbol(&anchored, DIFF_TEXT_COLUMN + 23, 0)
        );
    }

    fn text_columns(buffer: &Buffer, row_index: u16, wrap_width: u16) -> String {
        let mut line = String::new();
        let mut offset = 0;
        while offset < wrap_width {
            let symbol = symbol(buffer, DIFF_TEXT_COLUMN + offset, row_index);
            let width = symbol
                .chars()
                .next()
                .map(glyph_display_width)
                .unwrap_or(1)
                .max(1);
            line.push_str(&symbol);
            offset = offset.saturating_add(width);
        }
        line.trim_end_matches(' ').to_string()
    }

    #[test]
    fn cjk_wrapping_never_splits_a_wide_character() {
        let text = "文档内容中文说明";
        let rows = [row(DiffRowKind::Added, None, Some(1), text)];
        for wrap_width in [3u16, 4, 5, 7] {
            let display_rows = diff_display_rows(&rows, Some(wrap_width), CapsProfile::Rich);
            assert!(display_rows.len() > 1, "width {wrap_width} wraps the line");
            let row_count = u16::try_from(display_rows.len()).expect("the rows fit a u16");
            let buffer = render_wrapped(
                &rows,
                wrap_width,
                DIFF_TEXT_COLUMN + wrap_width,
                row_count,
                CapsProfile::Rich,
            );
            let mut rebuilt = String::new();
            for row_index in 0..row_count {
                let row_text = text_columns(&buffer, row_index, wrap_width);
                assert!(
                    !row_text.starts_with(' '),
                    "width {wrap_width} row {row_index} starts with a wide glyph spacer"
                );
                assert!(
                    usize::from(wrap_width) >= row_text.chars().count(),
                    "width {wrap_width} row {row_index} keeps {row_text:?} inside the pane"
                );
                rebuilt.push_str(&row_text);
            }
            assert_eq!(
                rebuilt, text,
                "width {wrap_width} keeps every wide character whole and in order"
            );
            assert_eq!(
                symbol(&buffer, DIFF_MARKER_COLUMN, 1),
                CONTINUATION_MARKER_UNICODE.to_string()
            );
            assert_eq!(
                symbol(&buffer, DIFF_OLD_NUMBER_COLUMN, 1),
                " ",
                "a continuation row leaves the line number column empty"
            );
        }
    }

    #[test]
    fn hunk_headers_do_not_wrap() {
        let text = "@@ -1,3 +1,3 @@ pub fn render_commit_details_pane(app: &App, area: Rect)";
        let rows = [hunk_header_row(0, text)];
        for wrap_width in [8u16, 20] {
            let display_rows = diff_display_rows(&rows, Some(wrap_width), CapsProfile::Rich);
            assert_eq!(
                display_rows.len(),
                1,
                "width {wrap_width} keeps a single hunk header row"
            );
            assert!(!display_rows[0].continuation);
        }
        let buffer = render_wrapped(&rows, 8, 96, 1, CapsProfile::Rich);
        assert_eq!(line_text(&buffer, 0), text);
    }

    #[test]
    fn wrapping_terminates_on_pathological_input() {
        let long_word = "x".repeat(500);
        let wide_text = "文档内容";
        let rows = [
            row(DiffRowKind::Added, None, Some(1), long_word.as_str()),
            row(DiffRowKind::Removed, Some(1), None, wide_text),
            row(DiffRowKind::Context, Some(2), Some(2), ""),
        ];
        for wrap_width in [1u16, 2, 3] {
            let display_rows = diff_display_rows(&rows, Some(wrap_width), CapsProfile::Basic);
            assert!(
                display_rows.len() >= long_word.chars().count() / usize::from(wrap_width),
                "width {wrap_width} puts at least one character on every row"
            );
            assert!(
                display_rows.len() <= long_word.chars().count() + wide_text.chars().count() + 1,
                "width {wrap_width} never grows past one row per character"
            );
            let last = display_rows.last().expect("the rows are never empty");
            assert_eq!(last.logical_row_index, 2);
        }
        let zero_width = diff_display_rows(&rows, Some(0), CapsProfile::Rich);
        assert_eq!(
            zero_width.len(),
            rows.len(),
            "a zero width pane falls back to one row per line"
        );
        let unwrapped = diff_display_rows(&rows, None, CapsProfile::Rich);
        assert_eq!(unwrapped.len(), rows.len());
    }

    #[test]
    fn continuation_markers_follow_the_caps_profile() {
        let rows = [row(DiffRowKind::Removed, Some(1), None, "abcdefghij")];
        let rich = render_wrapped(&rows, 4, DIFF_TEXT_COLUMN + 4, 3, CapsProfile::Rich);
        let standard = render_wrapped(&rows, 4, DIFF_TEXT_COLUMN + 4, 3, CapsProfile::Standard);
        let basic = render_wrapped(&rows, 4, DIFF_TEXT_COLUMN + 4, 3, CapsProfile::Basic);
        assert_eq!(
            symbol(&rich, DIFF_MARKER_COLUMN, 1),
            CONTINUATION_MARKER_UNICODE.to_string()
        );
        assert_eq!(
            symbol(&standard, DIFF_MARKER_COLUMN, 1),
            CONTINUATION_MARKER_UNICODE.to_string()
        );
        assert_eq!(
            symbol(&basic, DIFF_MARKER_COLUMN, 1),
            CONTINUATION_MARKER_ASCII.to_string()
        );
    }
}
