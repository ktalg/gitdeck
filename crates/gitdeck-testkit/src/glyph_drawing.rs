use crate::raster::RasterImage;

pub(crate) const CELL_PIXEL_WIDTH: u32 = 16;
pub(crate) const CELL_PIXEL_HEIGHT: u32 = 32;
pub(crate) const STROKE_PIXELS: u32 = 2;

const ASCII_GLYPH_LEFT_MARGIN: u32 = 1;
const ASCII_GLYPH_TOP_MARGIN: u32 = CELL_PIXEL_HEIGHT / 4;
const GLYPH_RASTER_SCALE: u32 = 2;
const GLYPH_PIXELS: u32 = 8;
const ASCII_GLYPH_WIDTH: u32 = GLYPH_PIXELS * GLYPH_RASTER_SCALE;
const ASCII_GLYPH_HEIGHT: u32 = GLYPH_PIXELS * GLYPH_RASTER_SCALE;
const ICON_CENTER_X: u32 = CELL_PIXEL_WIDTH / 2;
const ICON_CENTER_Y: u32 = CELL_PIXEL_HEIGHT / 2;
const ICON_RADIUS: u32 = 5;
const SINGLE_LINE_CENTER_X: u32 = (CELL_PIXEL_WIDTH - STROKE_PIXELS) / 2;
const SINGLE_LINE_CENTER_Y: u32 = (CELL_PIXEL_HEIGHT - STROKE_PIXELS) / 2;
const SINGLE_LINE_HORIZONTAL_ARM_LENGTH: u32 = CELL_PIXEL_WIDTH / 2 + STROKE_PIXELS / 2;
const SINGLE_LINE_VERTICAL_ARM_LENGTH: u32 = CELL_PIXEL_HEIGHT / 2 + STROKE_PIXELS / 2;
const DOUBLE_LINE_NEAR_X: u32 = 4;
const DOUBLE_LINE_FAR_X: u32 = 10;
const DOUBLE_LINE_TOP_Y: u32 = 9;
const DOUBLE_LINE_BOTTOM_Y: u32 = 21;
const MENU_RULE_OFFSETS: [u32; 3] = [ICON_CENTER_Y - 5, ICON_CENTER_Y - 1, ICON_CENTER_Y + 3];
const GEAR_RING_OUTER_RADIUS: i32 = 5;
const GEAR_RING_INNER_RADIUS: i32 = 3;
const GEAR_TOOTH_OFFSETS: [(i32, i32); 8] = [
    (6, 0),
    (4, 4),
    (0, 6),
    (-4, 4),
    (-6, 0),
    (-4, -4),
    (0, -6),
    (4, -4),
];

#[cfg(test)]
pub(crate) const BOX_DRAWING_GLYPHS: [char; 21] = [
    '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '═', '║', '╔', '╗', '╚', '╝', '╭', '╮',
    '╯', '╰',
];

pub(crate) const ROUNDED_CORNER_GLYPHS: [char; 4] = ['╭', '╮', '╯', '╰'];

pub(crate) const BLOCK_GLYPHS: [char; 8] = ['█', '▀', '▄', '▌', '▐', '░', '▒', '▓'];

pub(crate) const ICON_PIXEL_GLYPHS: [char; 14] = [
    '≡', '←', '▸', '▾', '●', '○', '◆', '✓', '✗', '⚠', '↑', '↓', '⌥', GEAR_GLYPH,
];

pub(crate) const GEAR_GLYPH: char = '⚙';

pub(crate) const ELLIPSIS_GLYPH: char = '…';
const ELLIPSIS_DOT_COLUMNS: [u32; 3] = [2, 7, 12];
const ELLIPSIS_DOT_ROW: u32 = ICON_CENTER_Y;
const ELLIPSIS_DOT_SIZE: u32 = STROKE_PIXELS + 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct NeighbourGlyphs {
    pub left: char,
    pub right: char,
    pub up: char,
    pub down: char,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ConnectorArms {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

pub(crate) fn ascii_connector_arms(character: char, neighbours: NeighbourGlyphs) -> ConnectorArms {
    let vertical_neighbour = |neighbour: char| matches!(neighbour, '|' | '+');
    let horizontal_neighbour = |neighbour: char| matches!(neighbour, '-' | '+');
    let any_connector = |neighbour: char| matches!(neighbour, '|' | '-' | '+');
    match character {
        '|' => ConnectorArms {
            up: vertical_neighbour(neighbours.up),
            down: vertical_neighbour(neighbours.down),
            ..ConnectorArms::default()
        },
        '-' => ConnectorArms {
            left: horizontal_neighbour(neighbours.left),
            right: horizontal_neighbour(neighbours.right),
            ..ConnectorArms::default()
        },
        '+' => ConnectorArms {
            left: any_connector(neighbours.left),
            right: any_connector(neighbours.right),
            up: any_connector(neighbours.up),
            down: any_connector(neighbours.down),
        },
        _ => ConnectorArms::default(),
    }
}

const LEFT: u8 = 0b0001;
const RIGHT: u8 = 0b0010;
const UP: u8 = 0b0100;
const DOWN: u8 = 0b1000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct LineArms {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

pub(crate) fn has_bundled_glyph(character: char) -> bool {
    line_arm_flags(character).is_some()
        || is_block_glyph(character)
        || is_icon_pixel_glyph(character)
        || character == ELLIPSIS_GLYPH
        || ascii_glyph_bitmap(character).is_some()
}

pub(crate) fn draw_glyph(
    image: &mut RasterImage,
    column: u16,
    row: u16,
    character: char,
    foreground: (u8, u8, u8),
    connector_arms: ConnectorArms,
) -> bool {
    if let Some(flags) = line_arm_flags(character) {
        let arms = LineArms::from_flags(flags);
        if is_double_line_character(character) {
            draw_double_line_arms(image, column, row, arms, foreground);
        } else if is_rounded_corner(character) {
            draw_rounded_corner_arms(image, column, row, arms, foreground);
        } else {
            draw_single_line_arms(image, column, row, arms, foreground);
        }
        return true;
    }
    if is_block_glyph(character) {
        draw_block_glyph(image, column, row, character, foreground);
        return true;
    }
    if is_icon_pixel_glyph(character) {
        draw_icon_glyph(image, column, row, character, foreground);
        return true;
    }
    if character == ELLIPSIS_GLYPH {
        draw_ellipsis(image, column, row, foreground);
        return true;
    }
    match ascii_glyph_bitmap(character) {
        Some(bitmap) => {
            draw_glyph_bitmap(image, column, row, &bitmap, foreground);
            draw_connector_strokes(image, column, row, connector_arms, foreground);
            true
        }
        None => false,
    }
}

pub(crate) fn draw_connector_strokes(
    image: &mut RasterImage,
    column: u16,
    row: u16,
    arms: ConnectorArms,
    foreground: (u8, u8, u8),
) {
    let origin_x = u32::from(column) * CELL_PIXEL_WIDTH;
    let origin_y = u32::from(row) * CELL_PIXEL_HEIGHT;
    let stroke_x = origin_x + SINGLE_LINE_CENTER_X;
    let stroke_y = origin_y + SINGLE_LINE_CENTER_Y;
    let band_bottom = ASCII_GLYPH_TOP_MARGIN + ASCII_GLYPH_HEIGHT;
    let band_right = ASCII_GLYPH_LEFT_MARGIN + ASCII_GLYPH_WIDTH;
    if arms.up {
        image.fill_rect(stroke_x, origin_y, STROKE_PIXELS, band_bottom, foreground);
    }
    if arms.down {
        image.fill_rect(
            stroke_x,
            origin_y + ASCII_GLYPH_TOP_MARGIN,
            STROKE_PIXELS,
            CELL_PIXEL_HEIGHT - ASCII_GLYPH_TOP_MARGIN,
            foreground,
        );
    }
    if arms.left {
        image.fill_rect(origin_x, stroke_y, band_right, STROKE_PIXELS, foreground);
    }
    if arms.right {
        image.fill_rect(
            origin_x + ASCII_GLYPH_LEFT_MARGIN,
            stroke_y,
            CELL_PIXEL_WIDTH - ASCII_GLYPH_LEFT_MARGIN,
            STROKE_PIXELS,
            foreground,
        );
    }
}

fn line_arm_flags(character: char) -> Option<u8> {
    let flags = match character {
        '─' | '═' => LEFT | RIGHT,
        '│' | '║' => UP | DOWN,
        '┌' | '╔' => RIGHT | DOWN,
        '┐' | '╗' => LEFT | DOWN,
        '└' | '╚' => RIGHT | UP,
        '┘' | '╝' => LEFT | UP,
        '╭' => RIGHT | DOWN,
        '╮' => LEFT | DOWN,
        '╯' => LEFT | UP,
        '╰' => RIGHT | UP,
        '├' => UP | DOWN | RIGHT,
        '┤' => UP | DOWN | LEFT,
        '┬' => LEFT | RIGHT | DOWN,
        '┴' => LEFT | RIGHT | UP,
        '┼' => LEFT | RIGHT | UP | DOWN,
        _ => return None,
    };
    Some(flags)
}

fn is_rounded_corner(character: char) -> bool {
    ROUNDED_CORNER_GLYPHS.contains(&character)
}

fn draw_rounded_corner_arms(
    image: &mut RasterImage,
    column: u16,
    row: u16,
    arms: LineArms,
    foreground: (u8, u8, u8),
) {
    let origin_x = u32::from(column) * CELL_PIXEL_WIDTH;
    let origin_y = u32::from(row) * CELL_PIXEL_HEIGHT;
    let corner_x = SINGLE_LINE_CENTER_X;
    let corner_y = SINGLE_LINE_CENTER_Y;
    let (horizontal_start, horizontal_width) = if arms.left {
        (0, corner_x)
    } else {
        (
            corner_x + STROKE_PIXELS,
            CELL_PIXEL_WIDTH - corner_x - STROKE_PIXELS,
        )
    };
    let (vertical_start, vertical_height) = if arms.up {
        (0, corner_y)
    } else {
        (
            corner_y + STROKE_PIXELS,
            CELL_PIXEL_HEIGHT - corner_y - STROKE_PIXELS,
        )
    };
    image.fill_rect(
        origin_x + horizontal_start,
        origin_y + corner_y,
        horizontal_width,
        STROKE_PIXELS,
        foreground,
    );
    image.fill_rect(
        origin_x + corner_x,
        origin_y + vertical_start,
        STROKE_PIXELS,
        vertical_height,
        foreground,
    );
    let chamfer_x = if arms.left {
        corner_x
    } else {
        corner_x + STROKE_PIXELS - 1
    };
    let chamfer_y = if arms.down {
        corner_y + STROKE_PIXELS - 1
    } else {
        corner_y - 1
    };
    image.fill_rect(origin_x + chamfer_x, origin_y + chamfer_y, 1, 1, foreground);
}

fn is_double_line_character(character: char) -> bool {
    matches!(character, '═' | '║' | '╔' | '╗' | '╚' | '╝')
}

impl LineArms {
    fn from_flags(flags: u8) -> Self {
        Self {
            left: flags & LEFT != 0,
            right: flags & RIGHT != 0,
            up: flags & UP != 0,
            down: flags & DOWN != 0,
        }
    }
}

fn draw_single_line_arms(
    image: &mut RasterImage,
    column: u16,
    row: u16,
    arms: LineArms,
    foreground: (u8, u8, u8),
) {
    let origin_x = u32::from(column) * CELL_PIXEL_WIDTH;
    let origin_y = u32::from(row) * CELL_PIXEL_HEIGHT;
    let horizontal_y = origin_y + SINGLE_LINE_CENTER_Y;
    let vertical_x = origin_x + SINGLE_LINE_CENTER_X;
    let horizontal = SINGLE_LINE_HORIZONTAL_ARM_LENGTH;
    let vertical = SINGLE_LINE_VERTICAL_ARM_LENGTH;
    let strokes = [
        (arms.left, origin_x, horizontal_y, horizontal, STROKE_PIXELS),
        (
            arms.right,
            vertical_x,
            horizontal_y,
            horizontal,
            STROKE_PIXELS,
        ),
        (arms.up, vertical_x, origin_y, STROKE_PIXELS, vertical),
        (arms.down, vertical_x, horizontal_y, STROKE_PIXELS, vertical),
    ];
    for (painted, stroke_x, stroke_y, width, height) in strokes {
        if painted {
            image.fill_rect(stroke_x, stroke_y, width, height, foreground);
        }
    }
}

fn draw_double_line_arms(
    image: &mut RasterImage,
    column: u16,
    row: u16,
    arms: LineArms,
    foreground: (u8, u8, u8),
) {
    let origin_x = u32::from(column) * CELL_PIXEL_WIDTH;
    let origin_y = u32::from(row) * CELL_PIXEL_HEIGHT;
    for rule_y in [DOUBLE_LINE_TOP_Y, DOUBLE_LINE_BOTTOM_Y] {
        draw_double_horizontal_rule(image, origin_x, origin_y, rule_y, arms, foreground);
    }
    for rule_x in [DOUBLE_LINE_NEAR_X, DOUBLE_LINE_FAR_X] {
        draw_double_vertical_rule(image, origin_x, origin_y, rule_x, arms, foreground);
    }
}

fn draw_double_horizontal_rule(
    image: &mut RasterImage,
    origin_x: u32,
    origin_y: u32,
    rule_y: u32,
    arms: LineArms,
    foreground: (u8, u8, u8),
) {
    if arms.left {
        image.fill_rect(
            origin_x,
            origin_y + rule_y,
            DOUBLE_LINE_NEAR_X + STROKE_PIXELS,
            STROKE_PIXELS,
            foreground,
        );
    }
    if arms.right {
        image.fill_rect(
            origin_x + DOUBLE_LINE_NEAR_X,
            origin_y + rule_y,
            CELL_PIXEL_WIDTH - DOUBLE_LINE_NEAR_X,
            STROKE_PIXELS,
            foreground,
        );
    }
}

fn draw_double_vertical_rule(
    image: &mut RasterImage,
    origin_x: u32,
    origin_y: u32,
    rule_x: u32,
    arms: LineArms,
    foreground: (u8, u8, u8),
) {
    if arms.up {
        image.fill_rect(
            origin_x + rule_x,
            origin_y,
            STROKE_PIXELS,
            DOUBLE_LINE_BOTTOM_Y + STROKE_PIXELS,
            foreground,
        );
    }
    if arms.down {
        image.fill_rect(
            origin_x + rule_x,
            origin_y + DOUBLE_LINE_TOP_Y,
            STROKE_PIXELS,
            CELL_PIXEL_HEIGHT - DOUBLE_LINE_TOP_Y,
            foreground,
        );
    }
}

fn is_block_glyph(character: char) -> bool {
    BLOCK_GLYPHS.contains(&character)
}

fn draw_block_glyph(
    image: &mut RasterImage,
    column: u16,
    row: u16,
    character: char,
    foreground: (u8, u8, u8),
) {
    let origin_x = u32::from(column) * CELL_PIXEL_WIDTH;
    let origin_y = u32::from(row) * CELL_PIXEL_HEIGHT;
    let half_width = CELL_PIXEL_WIDTH / 2;
    let half_height = CELL_PIXEL_HEIGHT / 2;
    let (offset_x, offset_y, width, height) = match character {
        '█' => (0, 0, CELL_PIXEL_WIDTH, CELL_PIXEL_HEIGHT),
        '▀' => (0, 0, CELL_PIXEL_WIDTH, half_height),
        '▄' => (
            0,
            half_height,
            CELL_PIXEL_WIDTH,
            CELL_PIXEL_HEIGHT - half_height,
        ),
        '▌' => (0, 0, half_width, CELL_PIXEL_HEIGHT),
        '▐' => (
            half_width,
            0,
            CELL_PIXEL_WIDTH - half_width,
            CELL_PIXEL_HEIGHT,
        ),
        _ => {
            fill_shaded_block(image, origin_x, origin_y, character, foreground);
            return;
        }
    };
    image.fill_rect(
        origin_x + offset_x,
        origin_y + offset_y,
        width,
        height,
        foreground,
    );
}

fn fill_shaded_block(
    image: &mut RasterImage,
    origin_x: u32,
    origin_y: u32,
    character: char,
    foreground: (u8, u8, u8),
) {
    for offset_y in 0..CELL_PIXEL_HEIGHT {
        for offset_x in 0..CELL_PIXEL_WIDTH {
            let on_quarter_lattice = offset_x % STROKE_PIXELS == 0 && offset_y % STROKE_PIXELS == 0;
            let painted = match character {
                '░' => on_quarter_lattice,
                '▒' => (offset_x + offset_y) % STROKE_PIXELS == 0,
                _ => !on_quarter_lattice,
            };
            if painted {
                image.set_pixel(origin_x + offset_x, origin_y + offset_y, foreground);
            }
        }
    }
}

fn is_icon_pixel_glyph(character: char) -> bool {
    ICON_PIXEL_GLYPHS.contains(&character)
}

fn draw_icon_glyph(
    image: &mut RasterImage,
    column: u16,
    row: u16,
    character: char,
    foreground: (u8, u8, u8),
) {
    let origin_x = u32::from(column) * CELL_PIXEL_WIDTH;
    let origin_y = u32::from(row) * CELL_PIXEL_HEIGHT;
    match character {
        '▸' => draw_triangle(
            image,
            origin_x,
            origin_y,
            TriangleDirection::Right,
            foreground,
        ),
        '▾' => draw_triangle(
            image,
            origin_x,
            origin_y,
            TriangleDirection::Down,
            foreground,
        ),
        '●' => draw_circle(image, origin_x, origin_y, true, foreground),
        '○' => draw_circle(image, origin_x, origin_y, false, foreground),
        '◆' => draw_diamond(image, origin_x, origin_y, foreground),
        '✓' => draw_check(image, origin_x, origin_y, foreground),
        '✗' => draw_cross(image, origin_x, origin_y, foreground),
        '⚠' => draw_warning(image, origin_x, origin_y, foreground),
        '↑' => draw_arrow(image, origin_x, origin_y, ArrowDirection::Up, foreground),
        '↓' => draw_arrow(image, origin_x, origin_y, ArrowDirection::Down, foreground),
        '←' => draw_arrow(image, origin_x, origin_y, ArrowDirection::Left, foreground),
        '≡' => draw_menu_rule(image, origin_x, origin_y, foreground),
        GEAR_GLYPH => draw_gear(image, origin_x, origin_y, foreground),
        _ => draw_option_key(image, origin_x, origin_y, foreground),
    }
}

fn draw_ellipsis(image: &mut RasterImage, column: u16, row: u16, foreground: (u8, u8, u8)) {
    let origin_x = u32::from(column) * CELL_PIXEL_WIDTH;
    let origin_y = u32::from(row) * CELL_PIXEL_HEIGHT;
    for dot_column in ELLIPSIS_DOT_COLUMNS {
        image.fill_rect(
            origin_x + dot_column,
            origin_y + ELLIPSIS_DOT_ROW,
            ELLIPSIS_DOT_SIZE,
            ELLIPSIS_DOT_SIZE,
            foreground,
        );
    }
}

fn draw_check(image: &mut RasterImage, origin_x: u32, origin_y: u32, foreground: (u8, u8, u8)) {
    draw_line(
        image,
        (origin_x + 3, origin_y + 15),
        (origin_x + 6, origin_y + 20),
        foreground,
    );
    draw_line(
        image,
        (origin_x + 6, origin_y + 20),
        (origin_x + 12, origin_y + 11),
        foreground,
    );
}

fn draw_cross(image: &mut RasterImage, origin_x: u32, origin_y: u32, foreground: (u8, u8, u8)) {
    draw_line(
        image,
        (origin_x + 4, origin_y + 12),
        (origin_x + 11, origin_y + 20),
        foreground,
    );
    draw_line(
        image,
        (origin_x + 11, origin_y + 12),
        (origin_x + 4, origin_y + 20),
        foreground,
    );
}

#[derive(Clone, Copy)]
enum TriangleDirection {
    Right,
    Down,
}

fn draw_triangle(
    image: &mut RasterImage,
    origin_x: u32,
    origin_y: u32,
    direction: TriangleDirection,
    foreground: (u8, u8, u8),
) {
    let size = ICON_RADIUS * 2 + 1;
    let half = ICON_RADIUS;
    for step in 0..size {
        let distance = step.abs_diff(half);
        let length = half - distance + 1;
        match direction {
            TriangleDirection::Right => {
                image.fill_rect(
                    origin_x + ICON_CENTER_X - half,
                    origin_y + ICON_CENTER_Y - half + step,
                    length,
                    1,
                    foreground,
                );
            }
            TriangleDirection::Down => {
                image.fill_rect(
                    origin_x + ICON_CENTER_X - half + step,
                    origin_y + ICON_CENTER_Y - half,
                    1,
                    length,
                    foreground,
                );
            }
        }
    }
}

fn draw_gear(image: &mut RasterImage, origin_x: u32, origin_y: u32, foreground: (u8, u8, u8)) {
    let center_x = origin_x + ICON_CENTER_X;
    let center_y = origin_y + ICON_CENTER_Y;
    let outer_squared = GEAR_RING_OUTER_RADIUS * GEAR_RING_OUTER_RADIUS;
    let inner_squared = GEAR_RING_INNER_RADIUS * GEAR_RING_INNER_RADIUS;
    for offset_y in -GEAR_RING_OUTER_RADIUS..=GEAR_RING_OUTER_RADIUS {
        for offset_x in -GEAR_RING_OUTER_RADIUS..=GEAR_RING_OUTER_RADIUS {
            let squared = offset_x * offset_x + offset_y * offset_y;
            if squared > outer_squared || squared < inner_squared {
                continue;
            }
            image.fill_rect(
                center_x.saturating_add_signed(offset_x),
                center_y.saturating_add_signed(offset_y),
                1,
                1,
                foreground,
            );
        }
    }
    for (offset_x, offset_y) in GEAR_TOOTH_OFFSETS {
        image.fill_rect(
            center_x.saturating_add_signed(offset_x),
            center_y.saturating_add_signed(offset_y),
            STROKE_PIXELS,
            STROKE_PIXELS,
            foreground,
        );
    }
}

fn draw_circle(
    image: &mut RasterImage,
    origin_x: u32,
    origin_y: u32,
    filled: bool,
    foreground: (u8, u8, u8),
) {
    let radius = ICON_RADIUS;
    let inner_radius = radius.saturating_sub(STROKE_PIXELS);
    for offset_y in 0..=radius * 2 {
        for offset_x in 0..=radius * 2 {
            let distance_x = offset_x as i32 - radius as i32;
            let distance_y = offset_y as i32 - radius as i32;
            let squared_distance = distance_x * distance_x + distance_y * distance_y;
            let inside_outer = squared_distance <= (radius * radius) as i32;
            let inside_inner = squared_distance < (inner_radius * inner_radius) as i32;
            let painted = if filled {
                inside_outer
            } else {
                inside_outer && !inside_inner
            };
            if painted {
                image.set_pixel(
                    origin_x + ICON_CENTER_X - radius + offset_x,
                    origin_y + ICON_CENTER_Y - radius + offset_y,
                    foreground,
                );
            }
        }
    }
}

fn draw_diamond(image: &mut RasterImage, origin_x: u32, origin_y: u32, foreground: (u8, u8, u8)) {
    let radius = ICON_RADIUS;
    for offset_y in 0..=radius * 2 {
        for offset_x in 0..=radius * 2 {
            let distance_x = (offset_x as i32 - radius as i32).unsigned_abs();
            let distance_y = (offset_y as i32 - radius as i32).unsigned_abs();
            if distance_x + distance_y <= radius {
                image.set_pixel(
                    origin_x + ICON_CENTER_X - radius + offset_x,
                    origin_y + ICON_CENTER_Y - radius + offset_y,
                    foreground,
                );
            }
        }
    }
}

fn draw_warning(image: &mut RasterImage, origin_x: u32, origin_y: u32, foreground: (u8, u8, u8)) {
    draw_line(
        image,
        (origin_x + 8, origin_y + 9),
        (origin_x + 2, origin_y + 21),
        foreground,
    );
    draw_line(
        image,
        (origin_x + 8, origin_y + 9),
        (origin_x + 14, origin_y + 21),
        foreground,
    );
    draw_line(
        image,
        (origin_x + 2, origin_y + 21),
        (origin_x + 14, origin_y + 21),
        foreground,
    );
    image.fill_rect(origin_x + 7, origin_y + 12, STROKE_PIXELS, 6, foreground);
    image.fill_rect(
        origin_x + 7,
        origin_y + 19,
        STROKE_PIXELS,
        STROKE_PIXELS,
        foreground,
    );
}

#[derive(Clone, Copy)]
enum ArrowDirection {
    Up,
    Down,
    Left,
}

fn draw_arrow(
    image: &mut RasterImage,
    origin_x: u32,
    origin_y: u32,
    direction: ArrowDirection,
    foreground: (u8, u8, u8),
) {
    let center_x = origin_x + ICON_CENTER_X;
    let center_y = origin_y + ICON_CENTER_Y;
    match direction {
        ArrowDirection::Up | ArrowDirection::Down => {
            let points_down = matches!(direction, ArrowDirection::Down);
            draw_vertical_arrow(image, center_x, center_y, points_down, foreground);
        }
        ArrowDirection::Left => draw_left_arrow(image, center_x, center_y, foreground),
    }
}

fn draw_vertical_arrow(
    image: &mut RasterImage,
    center_x: u32,
    center_y: u32,
    points_down: bool,
    foreground: (u8, u8, u8),
) {
    let reach = ICON_RADIUS;
    let head = STROKE_PIXELS * 2;
    image.fill_rect(
        center_x,
        center_y - reach,
        STROKE_PIXELS,
        reach * 2,
        foreground,
    );
    let tip_y = if points_down {
        center_y + reach
    } else {
        center_y - reach
    };
    let arm_y = if points_down {
        tip_y - head
    } else {
        tip_y + head
    };
    draw_line(
        image,
        (center_x, tip_y),
        (center_x - head, arm_y),
        foreground,
    );
    draw_line(
        image,
        (center_x + 1, tip_y),
        (center_x + 1 + head, arm_y),
        foreground,
    );
}

fn draw_left_arrow(
    image: &mut RasterImage,
    center_x: u32,
    center_y: u32,
    foreground: (u8, u8, u8),
) {
    let reach = ICON_RADIUS;
    let head = STROKE_PIXELS * 2;
    let tip_x = center_x - reach;
    image.fill_rect(tip_x, center_y, reach * 2, STROKE_PIXELS, foreground);
    draw_line(
        image,
        (tip_x, center_y),
        (tip_x + head, center_y - head),
        foreground,
    );
    draw_line(
        image,
        (tip_x, center_y + 1),
        (tip_x + head, center_y + 1 + head),
        foreground,
    );
}

fn draw_menu_rule(image: &mut RasterImage, origin_x: u32, origin_y: u32, foreground: (u8, u8, u8)) {
    for rule_y in MENU_RULE_OFFSETS {
        image.fill_rect(
            origin_x + 1,
            origin_y + rule_y,
            CELL_PIXEL_WIDTH - STROKE_PIXELS,
            STROKE_PIXELS,
            foreground,
        );
    }
}

fn draw_option_key(
    image: &mut RasterImage,
    origin_x: u32,
    origin_y: u32,
    foreground: (u8, u8, u8),
) {
    image.fill_rect(origin_x + 1, origin_y + 9, 6, STROKE_PIXELS, foreground);
    image.fill_rect(origin_x + 6, origin_y + 9, STROKE_PIXELS, 12, foreground);
    image.fill_rect(
        origin_x + 1,
        origin_y + 21,
        CELL_PIXEL_WIDTH - STROKE_PIXELS,
        STROKE_PIXELS,
        foreground,
    );
}

fn draw_line(image: &mut RasterImage, from: (u32, u32), to: (u32, u32), foreground: (u8, u8, u8)) {
    let delta_x = to.0 as i64 - from.0 as i64;
    let delta_y = to.1 as i64 - from.1 as i64;
    let steps = delta_x.abs().max(delta_y.abs()).max(1);
    for step in 0..=steps {
        let x = from.0 as i64 + delta_x * step / steps;
        let y = from.1 as i64 + delta_y * step / steps;
        image.fill_rect(
            u32::try_from(x).unwrap_or(0),
            u32::try_from(y).unwrap_or(0),
            STROKE_PIXELS,
            STROKE_PIXELS,
            foreground,
        );
    }
}

fn ascii_glyph_bitmap(character: char) -> Option<[u8; 8]> {
    if !character.is_ascii() || character.is_ascii_control() {
        return None;
    }
    use font8x8::UnicodeFonts;
    font8x8::BASIC_FONTS.get(character)
}

fn draw_glyph_bitmap(
    image: &mut RasterImage,
    column: u16,
    row: u16,
    bitmap: &[u8; 8],
    foreground: (u8, u8, u8),
) {
    let origin_x = u32::from(column) * CELL_PIXEL_WIDTH + ASCII_GLYPH_LEFT_MARGIN;
    let origin_y = u32::from(row) * CELL_PIXEL_HEIGHT + ASCII_GLYPH_TOP_MARGIN;
    for (glyph_row, bits) in bitmap.iter().enumerate() {
        for glyph_column in 0..GLYPH_PIXELS {
            if bits & (1 << glyph_column) == 0 {
                continue;
            }
            image.fill_rect(
                origin_x + glyph_column * GLYPH_RASTER_SCALE,
                origin_y + glyph_row as u32 * GLYPH_RASTER_SCALE,
                GLYPH_RASTER_SCALE,
                GLYPH_RASTER_SCALE,
                foreground,
            );
        }
    }
}
