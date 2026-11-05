use std::fs;
use std::path::Path;

use gitdeck_caps::ThemeId;
use ratatui::buffer::Buffer;

use crate::ansi_palette::resolve_color;
use crate::glyph_drawing::{ascii_connector_arms, draw_glyph, has_bundled_glyph, NeighbourGlyphs};
use crate::hashing::sha256_hex;
use crate::manifest::record_screenshot_artifact;
use crate::paths::screenshots_root;
use crate::screen_text::{buffer_text, symbol_columns};
use crate::screenshot_report::ScreenshotReport;
use crate::theme_palette::{theme_background, theme_foreground};

pub(crate) const CELL_PIXEL_WIDTH: u32 = 16;
pub(crate) const CELL_PIXEL_HEIGHT: u32 = 32;
const MISSING_GLYPH_BORDER_PIXELS: u32 = 1;

pub(crate) struct RasterImage {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixels: Vec<u8>,
}

impl RasterImage {
    fn filled(width: u32, height: u32, color: (u8, u8, u8)) -> Self {
        let mut pixels = Vec::with_capacity((width * height * 3) as usize);
        for _ in 0..width * height {
            pixels.extend_from_slice(&[color.0, color.1, color.2]);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    pub(crate) fn fill_rect(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: (u8, u8, u8),
    ) {
        for row in y..y.saturating_add(height) {
            for column in x..x.saturating_add(width) {
                self.set_pixel(column, row, color);
            }
        }
    }

    pub(crate) fn set_pixel(&mut self, x: u32, y: u32, color: (u8, u8, u8)) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = ((y * self.width + x) * 3) as usize;
        self.pixels[index] = color.0;
        self.pixels[index + 1] = color.1;
        self.pixels[index + 2] = color.2;
    }

    #[cfg(test)]
    fn pixel(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let index = ((y * self.width + x) * 3) as usize;
        (
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
        )
    }
}

pub(crate) fn raster_buffer(buffer: &Buffer, theme: ThemeId) -> RasterImage {
    let (image, _) = rasterize(buffer, theme_foreground(theme), theme_background(theme));
    image
}

pub(crate) fn missing_glyphs(buffer: &Buffer) -> Vec<char> {
    let mut missing_glyphs = Vec::new();
    for row in 0..buffer.area.height {
        let mut column = 0;
        while column < buffer.area.width {
            let Some(cell) = buffer.cell((column, row)) else {
                break;
            };
            let symbol = cell.symbol();
            if let Some(character) = symbol.chars().next().filter(|character| *character != ' ') {
                if !has_bundled_glyph(character) {
                    record_missing_glyph(&mut missing_glyphs, character);
                }
            }
            column += symbol_columns(symbol);
        }
    }
    missing_glyphs
}

pub(crate) fn encode_png(image: &RasterImage) -> Vec<u8> {
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, image.width, image.height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .unwrap_or_else(|error| panic!("cannot write png header: {error}"));
        writer
            .write_image_data(&image.pixels)
            .unwrap_or_else(|error| panic!("cannot write png data: {error}"));
        writer
            .finish()
            .unwrap_or_else(|error| panic!("cannot finish png: {error}"));
    }
    png_bytes
}

pub(crate) fn write_png_file(path: &Path, image: &RasterImage) {
    let png_bytes = encode_png(image);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
    }
    fs::write(path, png_bytes)
        .unwrap_or_else(|error| panic!("cannot write png {}: {error}", path.display()));
}

pub(crate) fn write_screenshot(buffer: &Buffer, theme: ThemeId, name: &str) -> ScreenshotReport {
    let default_foreground = theme_foreground(theme);
    let default_background = theme_background(theme);
    let (image, missing_glyphs) = rasterize(buffer, default_foreground, default_background);
    let screenshot_root = screenshots_root();
    let png_path = screenshot_root.join(format!("{name}.png"));
    let text_path = screenshot_root.join(format!("{name}.txt"));
    let ansi_path = screenshot_root.join(format!("{name}.ansi"));
    let png_bytes = encode_png(&image);
    write_screenshot_artifact(&png_path, &png_bytes);
    write_screenshot_artifact(&text_path, buffer_text(buffer).as_bytes());
    let ansi = buffer_ansi(buffer, default_foreground, default_background);
    write_screenshot_artifact(&ansi_path, ansi.as_bytes());
    ScreenshotReport {
        png_path,
        text_path,
        ansi_path,
        bytes_sha256: sha256_hex(&png_bytes),
        missing_glyphs,
    }
}

pub(crate) fn write_screenshot_into(
    directory: &Path,
    name: &str,
    buffer: &Buffer,
    theme: ThemeId,
) -> ScreenshotReport {
    let default_foreground = theme_foreground(theme);
    let default_background = theme_background(theme);
    let (image, missing_glyphs) = rasterize(buffer, default_foreground, default_background);
    let png_path = directory.join(format!("{name}.png"));
    let text_path = directory.join(format!("{name}.txt"));
    let ansi_path = directory.join(format!("{name}.ansi"));
    let png_bytes = encode_png(&image);
    write_loose_screenshot_artifact(&png_path, &png_bytes);
    write_loose_screenshot_artifact(&text_path, buffer_text(buffer).as_bytes());
    let ansi = buffer_ansi(buffer, default_foreground, default_background);
    write_loose_screenshot_artifact(&ansi_path, ansi.as_bytes());
    ScreenshotReport {
        png_path,
        text_path,
        ansi_path,
        bytes_sha256: sha256_hex(&png_bytes),
        missing_glyphs,
    }
}

fn write_loose_screenshot_artifact(artifact_path: &Path, bytes: &[u8]) {
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!(
                "cannot create screenshot directory {}: {error}",
                parent.display()
            )
        });
    }
    fs::write(artifact_path, bytes).unwrap_or_else(|error| {
        panic!(
            "cannot write screenshot artifact {}: {error}",
            artifact_path.display()
        )
    });
}

fn write_screenshot_artifact(artifact_path: &Path, bytes: &[u8]) {
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!(
                "cannot create screenshot directory {}: {error}",
                parent.display()
            )
        });
    }
    fs::write(artifact_path, bytes).unwrap_or_else(|error| {
        panic!(
            "cannot write screenshot artifact {}: {error}",
            artifact_path.display()
        )
    });
    record_screenshot_artifact(artifact_path);
}

fn rasterize(
    buffer: &Buffer,
    default_foreground: (u8, u8, u8),
    default_background: (u8, u8, u8),
) -> (RasterImage, Vec<char>) {
    let mut image = RasterImage::filled(
        u32::from(buffer.area.width) * CELL_PIXEL_WIDTH,
        u32::from(buffer.area.height) * CELL_PIXEL_HEIGHT,
        default_background,
    );
    let mut missing_glyphs = Vec::new();
    for row in 0..buffer.area.height {
        paint_cell_backgrounds(&mut image, buffer, row, default_background);
        paint_cell_glyphs(
            &mut image,
            buffer,
            row,
            default_foreground,
            default_background,
            &mut missing_glyphs,
        );
    }
    (image, missing_glyphs)
}

fn paint_cell_backgrounds(
    image: &mut RasterImage,
    buffer: &Buffer,
    row: u16,
    default_background: (u8, u8, u8),
) {
    let mut column = 0;
    while column < buffer.area.width {
        let Some(cell) = buffer.cell((column, row)) else {
            break;
        };
        let columns = symbol_columns(cell.symbol());
        let background = resolve_color(cell.bg, default_background);
        image.fill_rect(
            u32::from(column) * CELL_PIXEL_WIDTH,
            u32::from(row) * CELL_PIXEL_HEIGHT,
            u32::from(columns) * CELL_PIXEL_WIDTH,
            CELL_PIXEL_HEIGHT,
            background,
        );
        column += columns;
    }
}

fn paint_cell_glyphs(
    image: &mut RasterImage,
    buffer: &Buffer,
    row: u16,
    default_foreground: (u8, u8, u8),
    default_background: (u8, u8, u8),
    missing_glyphs: &mut Vec<char>,
) {
    let mut column = 0;
    while column < buffer.area.width {
        let Some(cell) = buffer.cell((column, row)) else {
            break;
        };
        let symbol = cell.symbol();
        let columns = symbol_columns(symbol);
        let foreground = resolve_color(cell.fg, default_foreground);
        let background = resolve_color(cell.bg, default_background);
        let Some(character) = symbol.chars().next().filter(|character| *character != ' ') else {
            column += columns;
            continue;
        };
        let connector_arms = ascii_connector_arms(
            character,
            NeighbourGlyphs {
                left: neighbour_symbol(buffer, column.checked_sub(1), Some(row)),
                right: neighbour_symbol(buffer, Some(column + columns), Some(row)),
                up: neighbour_symbol(buffer, Some(column), Some(row.wrapping_sub(1))),
                down: neighbour_symbol(buffer, Some(column), Some(row + 1)),
            },
        );
        if !draw_glyph(image, column, row, character, foreground, connector_arms) {
            record_missing_glyph(missing_glyphs, character);
            draw_missing_glyph(image, column, row, columns, foreground, background);
        }
        column += columns;
    }
}

fn neighbour_symbol(buffer: &Buffer, column: Option<u16>, row: Option<u16>) -> char {
    let (Some(column), Some(row)) = (column, row) else {
        return ' ';
    };
    buffer
        .cell((column, row))
        .and_then(|cell| cell.symbol().chars().next())
        .unwrap_or(' ')
}

fn draw_missing_glyph(
    image: &mut RasterImage,
    column: u16,
    row: u16,
    columns: u16,
    foreground: (u8, u8, u8),
    background: (u8, u8, u8),
) {
    let origin_x = u32::from(column) * CELL_PIXEL_WIDTH;
    let origin_y = u32::from(row) * CELL_PIXEL_HEIGHT;
    let box_width = u32::from(columns) * CELL_PIXEL_WIDTH;
    let border = MISSING_GLYPH_BORDER_PIXELS;
    image.fill_rect(origin_x, origin_y, box_width, CELL_PIXEL_HEIGHT, background);
    image.fill_rect(origin_x, origin_y, box_width, border, foreground);
    image.fill_rect(
        origin_x,
        origin_y + CELL_PIXEL_HEIGHT - border,
        box_width,
        border,
        foreground,
    );
    image.fill_rect(origin_x, origin_y, border, CELL_PIXEL_HEIGHT, foreground);
    image.fill_rect(
        origin_x + box_width - border,
        origin_y,
        border,
        CELL_PIXEL_HEIGHT,
        foreground,
    );
}

fn record_missing_glyph(missing_glyphs: &mut Vec<char>, character: char) {
    if !missing_glyphs.contains(&character) {
        missing_glyphs.push(character);
    }
}

fn buffer_ansi(
    buffer: &Buffer,
    default_foreground: (u8, u8, u8),
    default_background: (u8, u8, u8),
) -> String {
    let mut output = String::new();
    for row in 0..buffer.area.height {
        let mut column = 0;
        while column < buffer.area.width {
            let Some(cell) = buffer.cell((column, row)) else {
                break;
            };
            let foreground = resolve_color(cell.fg, default_foreground);
            let background = resolve_color(cell.bg, default_background);
            output.push_str(&format!(
                "\u{1b}[38;2;{};{};{}m\u{1b}[48;2;{};{};{}m{}",
                foreground.0,
                foreground.1,
                foreground.2,
                background.0,
                background.1,
                background.2,
                cell.symbol()
            ));
            column += symbol_columns(cell.symbol());
        }
        output.push_str("\u{1b}[0m\n");
    }
    output
}

#[cfg(test)]
mod raster_tests {
    use std::io::Cursor;

    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style};

    use super::{
        buffer_ansi, encode_png, missing_glyphs, rasterize, CELL_PIXEL_HEIGHT, CELL_PIXEL_WIDTH,
    };
    use crate::glyph_drawing::{BLOCK_GLYPHS, BOX_DRAWING_GLYPHS, ICON_PIXEL_GLYPHS};

    const MARKER_COLOR: (u8, u8, u8) = (0xDF, 0xE1, 0xE5);
    const ASCII_TEXT: &str = "abcdefg";

    fn rasterize_single_glyph(character: char, width: u16) -> (super::RasterImage, Vec<char>) {
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, 1));
        buffer.set_string(
            0,
            0,
            character.to_string(),
            Style::default().fg(Color::Rgb(MARKER_COLOR.0, MARKER_COLOR.1, MARKER_COLOR.2)),
        );
        rasterize(&buffer, (0, 0, 0), (0, 0, 0))
    }

    fn painted_rows(image: &super::RasterImage) -> Vec<u32> {
        (0..CELL_PIXEL_HEIGHT)
            .filter(|row| {
                (0..CELL_PIXEL_WIDTH).any(|column| image.pixel(column, *row) == MARKER_COLOR)
            })
            .collect()
    }

    fn painted_cell_columns(image: &super::RasterImage, column: u16) -> Vec<u32> {
        let origin_x = u32::from(column) * CELL_PIXEL_WIDTH;
        (0..CELL_PIXEL_WIDTH)
            .filter(|offset| {
                (0..CELL_PIXEL_HEIGHT)
                    .any(|row| image.pixel(origin_x + offset, row) == MARKER_COLOR)
            })
            .collect()
    }

    fn painted_cell_rows(image: &super::RasterImage, column: u16) -> Vec<u32> {
        let origin_x = u32::from(column) * CELL_PIXEL_WIDTH;
        (0..CELL_PIXEL_HEIGHT)
            .filter(|row| {
                (0..CELL_PIXEL_WIDTH)
                    .any(|offset| image.pixel(origin_x + offset, *row) == MARKER_COLOR)
            })
            .collect()
    }

    fn painted_columns(image: &super::RasterImage) -> Vec<u32> {
        (0..CELL_PIXEL_WIDTH)
            .filter(|column| {
                (0..CELL_PIXEL_HEIGHT).any(|row| image.pixel(*column, row) == MARKER_COLOR)
            })
            .collect()
    }

    #[test]
    fn vertical_border_glyph_covers_full_cell_height() {
        let (image, missing_glyphs) = rasterize_single_glyph('│', 1);
        assert!(missing_glyphs.is_empty());
        for row in 0..CELL_PIXEL_HEIGHT {
            let painted_columns = (0..CELL_PIXEL_WIDTH)
                .filter(|column| image.pixel(*column, row) == MARKER_COLOR)
                .count();
            assert!(
                painted_columns >= 2,
                "row {row} carries {painted_columns} painted pixels"
            );
        }
    }

    #[test]
    fn horizontal_border_glyph_covers_full_cell_width() {
        let (image, missing_glyphs) = rasterize_single_glyph('─', 1);
        assert!(missing_glyphs.is_empty());
        for column in 0..CELL_PIXEL_WIDTH {
            let painted_rows = (0..CELL_PIXEL_HEIGHT)
                .filter(|row| image.pixel(column, *row) == MARKER_COLOR)
                .count();
            assert!(
                painted_rows >= 2,
                "column {column} carries {painted_rows} painted pixels"
            );
        }
    }

    #[test]
    fn every_extra_glyph_renders_without_missing() {
        for character in ICON_PIXEL_GLYPHS {
            let (image, missing_glyphs) = rasterize_single_glyph(character, 1);
            assert!(missing_glyphs.is_empty(), "glyph {character:?}");
            assert!(
                !painted_rows(&image).is_empty(),
                "glyph {character:?} painted no pixels"
            );
        }
    }

    #[test]
    fn gear_glyph_renders_without_missing() {
        let (image, missing_glyphs) = rasterize_single_glyph('⚙', 1);
        assert!(missing_glyphs.is_empty(), "the gear glyph is bundled");
        assert_eq!(
            image.pixel(8, 16),
            (0, 0, 0),
            "the gear keeps a hollow centre"
        );
        let painted = painted_rows(&image);
        assert_eq!(painted.first(), Some(&10), "the gear keeps a top tooth");
        assert_eq!(painted.last(), Some(&23), "the gear keeps a bottom tooth");
        let columns = painted_columns(&image);
        assert_eq!(columns.first(), Some(&2), "the gear keeps a left tooth");
        assert_eq!(columns.last(), Some(&15), "the gear keeps a right tooth");
    }

    #[test]
    fn ascii_glyph_is_vertically_centered() {
        let (image, missing_glyphs) = rasterize_single_glyph('A', 1);
        assert!(missing_glyphs.is_empty());
        let rows = painted_rows(&image);
        let first_row = *rows.first().expect("glyph A painted rows");
        let last_row = *rows.last().expect("glyph A painted rows");
        assert!(
            (4..12).contains(&(first_row / 2)),
            "glyph A starts at half-cell row {}",
            first_row / 2
        );
        assert!(
            (4..12).contains(&(last_row / 2)),
            "glyph A ends at half-cell row {}",
            last_row / 2
        );
        assert!(
            (14..=18).contains(&((first_row + last_row) / 2)),
            "glyph A straddles the cell centre, rows {first_row}..{last_row}"
        );
    }

    #[test]
    fn ascii_glyphs_are_covered_by_the_ascii_table() {
        for character in ASCII_TEXT.chars() {
            let (image, missing_glyphs) = rasterize_single_glyph(character, 1);
            assert!(missing_glyphs.is_empty(), "ascii glyph {character:?}");
            assert!(!painted_rows(&image).is_empty());
        }
    }

    #[test]
    fn box_drawing_and_block_glyphs_are_drawn_without_missing() {
        for character in BOX_DRAWING_GLYPHS.into_iter().chain(BLOCK_GLYPHS) {
            let (_, missing_glyphs) = rasterize_single_glyph(character, 1);
            assert!(missing_glyphs.is_empty(), "glyph {character:?}");
        }
    }

    #[test]
    fn rounded_corner_glyphs_render_without_missing() {
        let corner_inset_pixels: [(char, (u32, u32)); 4] = [
            ('╭', (7, 15)),
            ('╮', (8, 15)),
            ('╯', (8, 16)),
            ('╰', (7, 16)),
        ];
        for (character, (column, row)) in corner_inset_pixels {
            let (image, missing_glyphs) = rasterize_single_glyph(character, 1);
            assert!(missing_glyphs.is_empty(), "glyph {character:?}");
            assert_eq!(
                image.pixel(column, row),
                (0, 0, 0),
                "glyph {character:?} keeps the rounded inset at ({column}, {row})"
            );
            assert!(
                !painted_rows(&image).is_empty(),
                "glyph {character:?} painted no pixels"
            );
        }
    }

    #[test]
    fn block_glyphs_paint_their_own_quadrant_only() {
        let (top_half, _) = rasterize_single_glyph('▀', 1);
        assert_eq!(
            painted_rows(&top_half),
            (0..CELL_PIXEL_HEIGHT / 2).collect::<Vec<u32>>()
        );
        let (left_half, _) = rasterize_single_glyph('▌', 1);
        assert_eq!(
            painted_columns(&left_half),
            (0..CELL_PIXEL_WIDTH / 2).collect::<Vec<u32>>()
        );
    }

    #[test]
    fn isolated_ascii_connectors_do_not_extend() {
        let mut dashed = Buffer::empty(Rect::new(0, 0, 3, 1));
        dashed.set_string(
            0,
            0,
            "a-b",
            Style::default().fg(Color::Rgb(MARKER_COLOR.0, MARKER_COLOR.1, MARKER_COLOR.2)),
        );
        let (dashed_image, _) = rasterize(&dashed, (0, 0, 0), (0, 0, 0));
        assert!(
            painted_cell_columns(&dashed_image, 1).len() <= 16,
            "an isolated hyphen must stay inside its glyph box"
        );
        assert!(painted_cell_rows(&dashed_image, 1).len() <= 16);

        let mut barred = Buffer::empty(Rect::new(0, 0, 7, 1));
        barred.set_string(
            0,
            0,
            " x | y ",
            Style::default().fg(Color::Rgb(MARKER_COLOR.0, MARKER_COLOR.1, MARKER_COLOR.2)),
        );
        let (barred_image, _) = rasterize(&barred, (0, 0, 0), (0, 0, 0));
        assert!(
            painted_cell_rows(&barred_image, 3).len() <= 16,
            "an isolated bar must stay inside its glyph box"
        );
        assert!(painted_cell_columns(&barred_image, 3).len() <= 16);
    }

    #[test]
    fn adjacent_ascii_connectors_extend_to_the_cell_edges() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 3));
        for row in 0..3 {
            buffer.set_string(
                0,
                row,
                "|",
                Style::default().fg(Color::Rgb(MARKER_COLOR.0, MARKER_COLOR.1, MARKER_COLOR.2)),
            );
        }
        let (image, _) = rasterize(&buffer, (0, 0, 0), (0, 0, 0));
        let middle_cell_rows = CELL_PIXEL_HEIGHT..2 * CELL_PIXEL_HEIGHT;
        let covered_rows = middle_cell_rows
            .filter(|row| {
                (0..CELL_PIXEL_WIDTH).any(|column| image.pixel(column, *row) == MARKER_COLOR)
            })
            .count();
        assert_eq!(
            covered_rows, CELL_PIXEL_HEIGHT as usize,
            "the middle bar of a stacked run covers every pixel row"
        );
    }

    #[test]
    fn cjk_glyphs_are_reported_as_missing() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
        buffer.set_string(0, 0, "中", Style::default());
        let (_, missing_glyphs) = rasterize(&buffer, (0, 0, 0), (0, 0, 0));
        assert_eq!(missing_glyphs, vec!['中']);
    }

    #[test]
    fn raster_image_covers_every_cell_at_double_scale() {
        let buffer = Buffer::empty(Rect::new(0, 0, 3, 2));
        let (image, _) = rasterize(&buffer, (0, 0, 0), (0, 0, 0));
        assert_eq!(image.width, 3 * CELL_PIXEL_WIDTH);
        assert_eq!(image.height, 2 * CELL_PIXEL_HEIGHT);
        assert_eq!(
            image.pixels.len(),
            (image.width * image.height * 3) as usize
        );
    }

    #[test]
    fn cell_background_color_fills_the_whole_cell_rect() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
        buffer.set_style(
            Rect::new(0, 0, 1, 1),
            Style::default().bg(Color::Rgb(10, 20, 30)),
        );
        let (image, _) = rasterize(&buffer, (0, 0, 0), (1, 2, 3));
        assert_eq!(&image.pixels[0..3], &[10, 20, 30]);
    }

    #[test]
    fn missing_glyph_renders_hollow_box_without_bleeding() {
        let left_background = (0x10, 0x11, 0x12);
        let missing_background = (0x20, 0x21, 0x22);
        let right_background = (0x30, 0x31, 0x32);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
        buffer.set_style(
            Rect::new(0, 0, 1, 1),
            Style::default().bg(Color::Rgb(0x10, 0x11, 0x12)),
        );
        buffer.set_style(
            Rect::new(1, 0, 1, 1),
            Style::default().bg(Color::Rgb(0x20, 0x21, 0x22)),
        );
        buffer.set_style(
            Rect::new(2, 0, 1, 1),
            Style::default().bg(Color::Rgb(0x30, 0x31, 0x32)),
        );
        buffer.set_string(
            1,
            0,
            "ñ",
            Style::default().fg(Color::Rgb(MARKER_COLOR.0, MARKER_COLOR.1, MARKER_COLOR.2)),
        );
        let (image, missing_glyphs) = rasterize(&buffer, (0, 0, 0), (0, 0, 0));
        assert_eq!(missing_glyphs, vec!['ñ']);

        let box_origin_x = CELL_PIXEL_WIDTH;
        let last_box_column = box_origin_x + CELL_PIXEL_WIDTH - 1;
        for row in 0..CELL_PIXEL_HEIGHT {
            assert_eq!(
                image.pixel(box_origin_x, row),
                MARKER_COLOR,
                "left box border breaks at row {row}"
            );
            assert_eq!(
                image.pixel(last_box_column, row),
                MARKER_COLOR,
                "right box border breaks at row {row}"
            );
        }
        for column in box_origin_x..=last_box_column {
            assert_eq!(
                image.pixel(column, 0),
                MARKER_COLOR,
                "top border at {column}"
            );
            assert_eq!(
                image.pixel(column, CELL_PIXEL_HEIGHT - 1),
                MARKER_COLOR,
                "bottom border at {column}"
            );
        }
        for row in 1..CELL_PIXEL_HEIGHT - 1 {
            for column in box_origin_x + 1..last_box_column {
                assert_eq!(
                    image.pixel(column, row),
                    missing_background,
                    "hollow box interior is not empty at ({column}, {row})"
                );
            }
        }
        for column in 0..CELL_PIXEL_WIDTH {
            for row in 0..CELL_PIXEL_HEIGHT {
                assert_eq!(
                    image.pixel(column, row),
                    left_background,
                    "the missing glyph paints into its left neighbour"
                );
                assert_eq!(
                    image.pixel(2 * CELL_PIXEL_WIDTH + column, row),
                    right_background,
                    "the missing glyph paints into its right neighbour"
                );
            }
        }
    }

    #[test]
    fn missing_glyph_box_covers_both_columns_of_a_wide_symbol() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
        buffer.set_string(
            0,
            0,
            "中",
            Style::default().fg(Color::Rgb(MARKER_COLOR.0, MARKER_COLOR.1, MARKER_COLOR.2)),
        );
        let (image, missing_glyphs) = rasterize(&buffer, (0, 0, 0), (0, 0, 0));
        assert_eq!(missing_glyphs, vec!['中']);
        let last_box_column = 2 * CELL_PIXEL_WIDTH - 1;
        for row in 0..CELL_PIXEL_HEIGHT {
            assert_eq!(image.pixel(last_box_column, row), MARKER_COLOR);
        }
        assert_eq!(
            image.pixel(last_box_column - 1, CELL_PIXEL_HEIGHT / 2),
            (0, 0, 0)
        );
    }

    #[test]
    fn missing_glyph_scan_ignores_covered_glyphs_and_blanks() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 1));
        buffer.set_string(0, 0, "a─ ", Style::default());
        buffer.set_string(3, 0, "中", Style::default());
        assert_eq!(missing_glyphs(&buffer), vec!['中']);
    }

    #[test]
    fn encoded_png_reports_cell_scaled_dimensions() {
        let buffer = Buffer::empty(Rect::new(0, 0, 3, 2));
        let (image, _) = rasterize(&buffer, (0, 0, 0), (0, 0, 0));
        let png_bytes = encode_png(&image);
        let decoder = png::Decoder::new(Cursor::new(png_bytes));
        let mut reader = decoder.read_info().expect("encoded png header");
        let mut pixels = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut pixels).expect("encoded png frame");
        assert_eq!(info.width, 3 * CELL_PIXEL_WIDTH);
        assert_eq!(info.height, 2 * CELL_PIXEL_HEIGHT);
        assert_eq!(info.color_type, png::ColorType::Rgb);
    }

    #[test]
    fn ansi_output_emits_truecolor_sequences_per_cell() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
        buffer.set_string(
            0,
            0,
            "ab",
            Style::default()
                .fg(Color::Rgb(1, 2, 3))
                .bg(Color::Rgb(4, 5, 6)),
        );
        let ansi = buffer_ansi(&buffer, (0, 0, 0), (0, 0, 0));
        assert!(ansi.contains("\u{1b}[38;2;1;2;3m"));
        assert!(ansi.contains("\u{1b}[48;2;4;5;6m"));
        assert!(ansi.ends_with("\u{1b}[0m\n"));
    }
}
