use std::env;
use std::fs;
use std::path::Path;

use gitdeck_caps::ThemeId;
use ratatui::buffer::Buffer;

use crate::paths::{golden_root, repository_relative_path, screenshots_root};
use crate::raster::{raster_buffer, write_png_file, RasterImage};

const WRITE_GOLDEN_ENVIRONMENT_VARIABLE: &str = "GITDECK_WRITE_GOLDEN";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelTolerance {
    pub max_channel_delta: u8,
    pub max_pixel_ratio: f32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PixelDiff {
    pub differing_pixels: usize,
    pub total_pixels: usize,
    pub max_channel_delta: u8,
}

pub(crate) fn compare_with_golden(
    buffer: &Buffer,
    theme: ThemeId,
    golden: &str,
    tolerance: PixelTolerance,
) -> PixelDiff {
    let current = raster_buffer(buffer, theme);
    let golden_path = golden_root().join(format!("{golden}.png"));
    if writes_golden() {
        write_png_file(&golden_path, &current);
        println!("wrote golden {}", golden_path.display());
        return PixelDiff {
            differing_pixels: 0,
            total_pixels: usize::try_from(current.width * current.height).unwrap_or(0),
            max_channel_delta: 0,
        };
    }
    if !golden_path.is_file() {
        panic!(
            "golden {} is missing; set {} =1 to record it",
            repository_relative_path(&golden_path).display(),
            WRITE_GOLDEN_ENVIRONMENT_VARIABLE
        );
    }
    let (golden_width, golden_height, golden_pixels) = decode_png(&golden_path);
    assert_eq!(
        (golden_width, golden_height),
        (current.width, current.height),
        "golden {} measures {golden_width}x{golden_height} while the frame measures {}x{}",
        repository_relative_path(&golden_path).display(),
        current.width,
        current.height
    );
    let diff = measure_pixel_difference(&current, &golden_pixels, tolerance);
    let differing_ratio = ratio(diff.differing_pixels, diff.total_pixels);
    if differing_ratio > tolerance.max_pixel_ratio {
        let diff_path = screenshots_root()
            .join("diff")
            .join(format!("{golden}.png"));
        write_png_file(
            &diff_path,
            &difference_image(&current, &golden_pixels, tolerance),
        );
        panic!(
            "golden {} differs on {}/{} pixels ({:.4} > {:.4}), max channel delta {} (tolerance {})",
            repository_relative_path(&golden_path).display(),
            diff.differing_pixels,
            diff.total_pixels,
            differing_ratio,
            tolerance.max_pixel_ratio,
            diff.max_channel_delta,
            tolerance.max_channel_delta
        );
    }
    diff
}

fn writes_golden() -> bool {
    env::var(WRITE_GOLDEN_ENVIRONMENT_VARIABLE)
        .map(|value| value == "1")
        .unwrap_or(false)
}

fn measure_pixel_difference(
    current: &RasterImage,
    golden_pixels: &[u8],
    tolerance: PixelTolerance,
) -> PixelDiff {
    let mut differing_pixels = 0;
    let mut max_channel_delta = 0;
    for (current_pixel, golden_pixel) in current.pixels.chunks(3).zip(golden_pixels.chunks(3)) {
        let channel_delta = current_pixel
            .iter()
            .zip(golden_pixel)
            .map(|(current_channel, golden_channel)| current_channel.abs_diff(*golden_channel))
            .max()
            .unwrap_or(0);
        max_channel_delta = max_channel_delta.max(channel_delta);
        if channel_delta > tolerance.max_channel_delta {
            differing_pixels += 1;
        }
    }
    PixelDiff {
        differing_pixels,
        total_pixels: usize::try_from(current.width * current.height).unwrap_or(0),
        max_channel_delta,
    }
}

fn ratio(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        return 0.0;
    }
    numerator as f32 / denominator as f32
}

fn difference_image(
    current: &RasterImage,
    golden_pixels: &[u8],
    tolerance: PixelTolerance,
) -> RasterImage {
    let mut image = RasterImage {
        width: current.width,
        height: current.height,
        pixels: current.pixels.clone(),
    };
    for (index, (current_pixel, golden_pixel)) in current
        .pixels
        .chunks(3)
        .zip(golden_pixels.chunks(3))
        .enumerate()
    {
        let channel_delta = current_pixel
            .iter()
            .zip(golden_pixel)
            .map(|(current_channel, golden_channel)| current_channel.abs_diff(*golden_channel))
            .max()
            .unwrap_or(0);
        if channel_delta > tolerance.max_channel_delta {
            let column = u32::try_from(index % current.width as usize).unwrap_or(0);
            let row = u32::try_from(index / current.width as usize).unwrap_or(0);
            image.set_pixel(column, row, (0xFF, 0x00, 0x00));
        }
    }
    image
}

fn decode_png(path: &Path) -> (u32, u32, Vec<u8>) {
    let file = fs::File::open(path)
        .unwrap_or_else(|error| panic!("cannot open golden {}: {error}", path.display()));
    let decoder = png::Decoder::new(file);
    let mut reader = decoder
        .read_info()
        .unwrap_or_else(|error| panic!("cannot read golden header {}: {error}", path.display()));
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut pixels)
        .unwrap_or_else(|error| panic!("cannot decode golden {}: {error}", path.display()));
    assert_eq!(
        (info.color_type, info.bit_depth),
        (png::ColorType::Rgb, png::BitDepth::Eight),
        "golden {} must be 8-bit RGB",
        path.display()
    );
    pixels.truncate(info.buffer_size());
    (info.width, info.height, pixels)
}
