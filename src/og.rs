use std::{error::Error, fs, path::Path};

use fontdue::{Font, FontSettings};
use image::{imageops::FilterType, Rgba, RgbaImage};

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 630;
const PADDING_X: i32 = 104;
const TITLE_BASELINE_START: i32 = 150;
const TITLE_SIZE: f32 = 64.0;
const TITLE_LINE_HEIGHT: i32 = 78;
const TITLE_MAX_LINES: usize = 3;
const DESC_SIZE: f32 = 30.0;
const DESC_LINE_HEIGHT: i32 = 44;
const DESC_MAX_LINES: usize = 3;
const URL_BASELINE: i32 = 560;
const URL_SIZE: f32 = 28.0;

const COLOR_DARK: Rgba<u8> = Rgba([26, 26, 30, 255]);
const COLOR_GRAY: Rgba<u8> = Rgba([111, 111, 120, 255]);
const COLOR_PRIMARY: Rgba<u8> = Rgba([240, 0, 72, 255]);

static MEDIUM_FONT: &[u8] = include_bytes!("fonts/inter-medium.ttf");
static REGULAR_FONT: &[u8] = include_bytes!("fonts/inter-regular.ttf");

pub fn render(
    bg_path: &Path,
    title: &str,
    description: &str,
    site_line: &str,
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let bg_bytes = fs::read(bg_path)
        .map_err(|e| format!("failed to read the og background asset: {e}"))?;
    let bg = image::load_from_memory(&bg_bytes)
        .map_err(|e| format!("failed to decode the og background asset: {e}"))?;
    let mut canvas = bg
        .resize_exact(WIDTH, HEIGHT, FilterType::Lanczos3)
        .to_rgba8();

    let medium_font = Font::from_bytes(MEDIUM_FONT, FontSettings::default())?;
    let regular_font = Font::from_bytes(REGULAR_FONT, FontSettings::default())?;

    let max_width = (WIDTH as i32 - PADDING_X * 2) as f32;

    let title_lines = wrap(&medium_font, title, TITLE_SIZE, max_width, TITLE_MAX_LINES);
    let mut baseline = TITLE_BASELINE_START;
    for line in &title_lines {
        draw_line(&mut canvas, &medium_font, line, TITLE_SIZE, PADDING_X, baseline, COLOR_DARK);
        baseline += TITLE_LINE_HEIGHT;
    }

    let desc_lines = wrap(&regular_font, description, DESC_SIZE, max_width, DESC_MAX_LINES);
    let mut desc_baseline = baseline + 12;
    for line in &desc_lines {
        draw_line(&mut canvas, &regular_font, line, DESC_SIZE, PADDING_X, desc_baseline, COLOR_GRAY);
        desc_baseline += DESC_LINE_HEIGHT;
    }

    draw_line(&mut canvas, &medium_font, site_line, URL_SIZE, PADDING_X, URL_BASELINE, COLOR_PRIMARY);

    let rgb = image::DynamicImage::ImageRgba8(canvas).to_rgb8();
    let file = std::fs::File::create(out_path)
        .map_err(|e| format!("failed to create the og image: {e}"))?;
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&file, 85);
    rgb.write_with_encoder(encoder)
        .map_err(|e| format!("failed to encode the og image: {e}"))?;
    Ok(())
}

fn measure(font: &Font, text: &str, px: f32) -> f32 {
    text.chars().fold(0.0, |width, ch| width + font.metrics(ch, px).advance_width)
}

fn wrap(font: &Font, text: &str, px: f32, max_width: f32, max_lines: usize) -> Vec<String> {
    let space_width = font.metrics(' ', px).advance_width;
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut line_width = 0.0;

    for word in text.split_whitespace() {
        let word_width = measure(font, word, px);
        let added = if line.is_empty() { word_width } else { space_width + word_width };
        if !line.is_empty() && line_width + added > max_width {
            lines.push(std::mem::take(&mut line));
            line_width = 0.0;
        }
        if !line.is_empty() {
            line.push(' ');
            line_width += space_width;
        }
        line.push_str(word);
        line_width += word_width;
    }
    if !line.is_empty() {
        lines.push(line);
    }

    if lines.len() > max_lines {
        lines.truncate(max_lines);
        let last = lines.last_mut().expect("max_lines >= 1");
        while !last.is_empty() && measure(font, &format!("{last}\u{2026}"), px) > max_width {
            last.pop();
        }
        last.push('\u{2026}');
    }
    lines
}

fn draw_line(
    canvas: &mut RgbaImage,
    font: &Font,
    text: &str,
    px: f32,
    x: i32,
    baseline: i32,
    color: Rgba<u8>,
) {
    let mut pen_x = x as f32;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, px);
        let glyph_top = baseline - (metrics.ymin + metrics.height as i32);
        for row in 0..metrics.height {
            let y = glyph_top + row as i32;
            if y < 0 || y >= HEIGHT as i32 {
                continue;
            }
            for col in 0..metrics.width {
                let alpha = bitmap[row * metrics.width + col];
                if alpha == 0 {
                    continue;
                }
                let gx = pen_x as i32 + metrics.xmin + col as i32;
                if gx < 0 || gx >= WIDTH as i32 {
                    continue;
                }
                blend(canvas, gx as u32, y as u32, color, alpha);
            }
        }
        pen_x += metrics.advance_width;
    }
}

fn blend(canvas: &mut RgbaImage, x: u32, y: u32, color: Rgba<u8>, alpha: u8) {
    let pixel = canvas.get_pixel_mut(x, y);
    let a = alpha as u32;
    let inv = 255 - a;
    pixel.0[0] = ((pixel.0[0] as u32 * inv + color.0[0] as u32 * a) / 255) as u8;
    pixel.0[1] = ((pixel.0[1] as u32 * inv + color.0[1] as u32 * a) / 255) as u8;
    pixel.0[2] = ((pixel.0[2] as u32 * inv + color.0[2] as u32 * a) / 255) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_to_line_budget_and_ellipsizes() {
        let font = Font::from_bytes(MEDIUM_FONT, FontSettings::default()).unwrap();
        let short = wrap(&font, "Hello, world", 64.0, 900.0, 3);
        assert_eq!(short.len(), 1);
        let long = "word ".repeat(80);
        let lines = wrap(&font, long.trim(), 64.0, 900.0, 3);
        assert_eq!(lines.len(), 3);
        assert!(lines[2].ends_with('\u{2026}'));
    }

    #[test]
    fn renders_a_card_to_jpg() {
        let dir = std::env::temp_dir().join("main-tests-og");
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("og.jpg");
        render(
            Path::new("src/bg.png"),
            "Your project has a light cone",
            "Before anyone, human or agent, touches a line of code, you can compute exactly what must exist first.",
            "alxshelepenok.com",
            &out,
        )
        .unwrap();
        let img = image::open(&out).unwrap();
        assert_eq!((img.width(), img.height()), (1200, 630));
    }
}
