//! Deterministic visual storytelling for DesignPatch review GIFs.

use opencad_core::{OpenCadError, Result};
use opencad_graph::{DesignDiff, SemanticChange};
use opencad_render::RenderImage;

use crate::review::EffectCheck;

const INK: [u8; 3] = [236, 244, 255];
const MUTED: [u8; 3] = [166, 181, 204];
const BLUE: [u8; 3] = [65, 168, 255];
const AMBER: [u8; 3] = [255, 172, 82];
const GREEN: [u8; 3] = [83, 210, 145];
const PANEL: [u8; 3] = [12, 18, 29];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewGifLabels {
    before_parameter: String,
    after_parameter: String,
    parameter_change: String,
    mass_change: Option<String>,
    checks: String,
    checks_passed: bool,
}

impl ReviewGifLabels {
    pub(crate) fn from_review(diff: &DesignDiff, effects: &[EffectCheck]) -> Self {
        let parameter = diff.changes.iter().find_map(|change| match change {
            SemanticChange::ParameterChanged { id, before, after } => {
                let name = id
                    .rsplit(':')
                    .next()
                    .unwrap_or(id)
                    .replace(['_', '-'], " ")
                    .to_ascii_uppercase();
                Some((
                    name,
                    before.to_ascii_uppercase(),
                    after.to_ascii_uppercase(),
                ))
            }
            _ => None,
        });
        let (before_parameter, after_parameter, parameter_change) = parameter.map_or_else(
            || {
                (
                    "ORIGINAL DESIGN".into(),
                    "PATCHED DESIGN".into(),
                    "SEMANTIC CHANGE VERIFIED".into(),
                )
            },
            |(name, before, after)| {
                (
                    format!("{name}  {before}"),
                    format!("{name}  {after}"),
                    format!("{name}  {before} > {after}"),
                )
            },
        );
        let mass_change = diff.changes.iter().find_map(|change| match change {
            SemanticChange::MassChanged { before, after } => Some(format!(
                "MASS  {} > {}",
                before.to_ascii_uppercase(),
                after.to_ascii_uppercase()
            )),
            _ => None,
        });
        let passed = effects.iter().filter(|effect| effect.passed).count();
        let total = effects.len();
        let checks_passed = passed == total;
        let checks = if total == 0 {
            "VALIDATION COMPLETE".into()
        } else if checks_passed {
            format!("{passed}/{total} CHECKS PASS")
        } else {
            format!("{passed}/{total} CHECKS PASS / REVIEW")
        };
        Self {
            before_parameter,
            after_parameter,
            parameter_change,
            mass_change,
            checks,
            checks_passed,
        }
    }
}

pub(crate) fn comparison_frames(
    before: &RenderImage,
    after: &RenderImage,
    labels: &ReviewGifLabels,
    frames_per_second: u32,
) -> Result<Vec<RenderImage>> {
    if before.width != after.width || before.height != after.height {
        return Err(OpenCadError::validation(
            "review GIF images must have identical pixel dimensions",
        ));
    }
    if frames_per_second == 0 {
        return Err(OpenCadError::validation(
            "review GIF frame rate must be positive",
        ));
    }

    let stage_frames = frames_per_second as usize;
    let mut frames = Vec::with_capacity(stage_frames * 5);
    for _ in 0..stage_frames {
        frames.push(decorate_single(
            before.clone(),
            "BEFORE",
            &labels.before_parameter,
            "DESIGN GRAPH SOURCE",
            BLUE,
        ));
    }
    for index in 0..stage_frames {
        let mut frame = wipe_frame(before, after, index + 1, stage_frames + 1);
        decorate_chrome(&mut frame, "DRY RUN + REGEN", AMBER);
        draw_footer(
            &mut frame,
            &labels.parameter_change,
            "TRANSACTIONAL / SOURCE UNCHANGED",
            AMBER,
        );
        frames.push(frame);
    }
    for _ in 0..stage_frames {
        frames.push(decorate_single(
            after.clone(),
            "AFTER",
            &labels.after_parameter,
            &labels.checks,
            if labels.checks_passed { GREEN } else { AMBER },
        ));
    }
    for _ in 0..stage_frames * 2 {
        frames.push(split_frame(before, after, labels));
    }
    Ok(frames)
}

fn decorate_single(
    mut frame: RenderImage,
    stage: &str,
    primary: &str,
    secondary: &str,
    accent: [u8; 3],
) -> RenderImage {
    decorate_chrome(&mut frame, stage, accent);
    draw_footer(&mut frame, primary, secondary, accent);
    frame
}

fn split_frame(before: &RenderImage, after: &RenderImage, labels: &ReviewGifLabels) -> RenderImage {
    let mut frame = before.clone();
    let middle = frame.width / 2;
    let width = frame.width;
    let content_height = frame.height.saturating_sub(116);
    copy_region(after, &mut frame, middle, width);
    fill_rect(
        &mut frame,
        middle.saturating_sub(2),
        48,
        4,
        content_height,
        INK,
        230,
    );
    decorate_chrome(&mut frame, "VERIFIED DIFF", GREEN);
    badge(&mut frame, 18, 62, "BEFORE", BLUE);
    badge(&mut frame, middle + 18, 62, "AFTER", GREEN);
    let secondary = labels.mass_change.as_deref().unwrap_or(&labels.checks);
    draw_footer(
        &mut frame,
        &labels.parameter_change,
        secondary,
        if labels.checks_passed { GREEN } else { AMBER },
    );
    frame
}

fn wipe_frame(
    before: &RenderImage,
    after: &RenderImage,
    numerator: usize,
    denominator: usize,
) -> RenderImage {
    let mut frame = before.clone();
    let boundary = (frame.width as usize * numerator / denominator) as u32;
    let content_height = frame.height.saturating_sub(116);
    copy_region(after, &mut frame, 0, boundary);
    fill_rect(
        &mut frame,
        boundary.saturating_sub(3),
        48,
        6,
        content_height,
        AMBER,
        235,
    );
    frame
}

fn copy_region(source: &RenderImage, target: &mut RenderImage, start_x: u32, end_x: u32) {
    let end_x = end_x.min(target.width);
    for y in 0..target.height {
        for x in start_x.min(end_x)..end_x {
            let index = ((y * target.width + x) * 4) as usize;
            target.rgba[index..index + 4].copy_from_slice(&source.rgba[index..index + 4]);
        }
    }
}

fn decorate_chrome(frame: &mut RenderImage, stage: &str, accent: [u8; 3]) {
    fill_rect(frame, 0, 0, frame.width, 48, PANEL, 225);
    fill_rect(frame, 0, 47, frame.width, 1, accent, 255);
    draw_text(frame, 20, 17, "MUSUBICAD / DESIGN PATCH REVIEW", 2, INK);
    let badge_width = text_width(stage, 2) + 24;
    badge(
        frame,
        frame.width.saturating_sub(badge_width + 18),
        11,
        stage,
        accent,
    );
}

fn draw_footer(frame: &mut RenderImage, primary: &str, secondary: &str, accent: [u8; 3]) {
    let top = frame.height.saturating_sub(68);
    fill_rect(frame, 0, top, frame.width, 68, PANEL, 225);
    fill_rect(frame, 0, top, 7, 68, accent, 255);
    draw_text(frame, 22, top + 13, primary, 2, INK);
    draw_text(frame, 22, top + 40, secondary, 1, MUTED);
}

fn badge(frame: &mut RenderImage, x: u32, y: u32, text: &str, color: [u8; 3]) {
    let width = text_width(text, 2) + 16;
    fill_rect(frame, x, y, width, 25, color, 225);
    draw_text(frame, x + 8, y + 6, text, 2, PANEL);
}

fn text_width(text: &str, scale: u32) -> u32 {
    text.chars().count() as u32 * 6 * scale
}

fn draw_text(
    frame: &mut RenderImage,
    origin_x: u32,
    origin_y: u32,
    text: &str,
    scale: u32,
    color: [u8; 3],
) {
    let mut cursor_x = origin_x;
    for character in text.chars() {
        let glyph = glyph(character.to_ascii_uppercase());
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    fill_rect(
                        frame,
                        cursor_x + column * scale,
                        origin_y + row as u32 * scale,
                        scale,
                        scale,
                        color,
                        255,
                    );
                }
            }
        }
        cursor_x += 6 * scale;
        if cursor_x >= frame.width {
            break;
        }
    }
}

fn fill_rect(
    frame: &mut RenderImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: [u8; 3],
    alpha: u8,
) {
    let end_x = x.saturating_add(width).min(frame.width);
    let end_y = y.saturating_add(height).min(frame.height);
    let alpha = u16::from(alpha);
    for pixel_y in y.min(end_y)..end_y {
        for pixel_x in x.min(end_x)..end_x {
            let index = ((pixel_y * frame.width + pixel_x) * 4) as usize;
            for (channel, value) in color.iter().enumerate() {
                let existing = u16::from(frame.rgba[index + channel]);
                frame.rgba[index + channel] =
                    ((existing * (255 - alpha) + u16::from(*value) * alpha) / 255) as u8;
            }
            frame.rgba[index + 3] = 255;
        }
    }
}

#[rustfmt::skip]
fn glyph(character: char) -> [u8; 7] {
    match character {
        'A' => [0b01110,0b10001,0b10001,0b11111,0b10001,0b10001,0b10001],
        'B' => [0b11110,0b10001,0b10001,0b11110,0b10001,0b10001,0b11110],
        'C' => [0b01111,0b10000,0b10000,0b10000,0b10000,0b10000,0b01111],
        'D' => [0b11110,0b10001,0b10001,0b10001,0b10001,0b10001,0b11110],
        'E' => [0b11111,0b10000,0b10000,0b11110,0b10000,0b10000,0b11111],
        'F' => [0b11111,0b10000,0b10000,0b11110,0b10000,0b10000,0b10000],
        'G' => [0b01111,0b10000,0b10000,0b10111,0b10001,0b10001,0b01111],
        'H' => [0b10001,0b10001,0b10001,0b11111,0b10001,0b10001,0b10001],
        'I' => [0b11111,0b00100,0b00100,0b00100,0b00100,0b00100,0b11111],
        'J' => [0b00111,0b00010,0b00010,0b00010,0b10010,0b10010,0b01100],
        'K' => [0b10001,0b10010,0b10100,0b11000,0b10100,0b10010,0b10001],
        'L' => [0b10000,0b10000,0b10000,0b10000,0b10000,0b10000,0b11111],
        'M' => [0b10001,0b11011,0b10101,0b10101,0b10001,0b10001,0b10001],
        'N' => [0b10001,0b11001,0b10101,0b10011,0b10001,0b10001,0b10001],
        'O' => [0b01110,0b10001,0b10001,0b10001,0b10001,0b10001,0b01110],
        'P' => [0b11110,0b10001,0b10001,0b11110,0b10000,0b10000,0b10000],
        'Q' => [0b01110,0b10001,0b10001,0b10001,0b10101,0b10010,0b01101],
        'R' => [0b11110,0b10001,0b10001,0b11110,0b10100,0b10010,0b10001],
        'S' => [0b01111,0b10000,0b10000,0b01110,0b00001,0b00001,0b11110],
        'T' => [0b11111,0b00100,0b00100,0b00100,0b00100,0b00100,0b00100],
        'U' => [0b10001,0b10001,0b10001,0b10001,0b10001,0b10001,0b01110],
        'V' => [0b10001,0b10001,0b10001,0b10001,0b10001,0b01010,0b00100],
        'W' => [0b10001,0b10001,0b10001,0b10101,0b10101,0b10101,0b01010],
        'X' => [0b10001,0b10001,0b01010,0b00100,0b01010,0b10001,0b10001],
        'Y' => [0b10001,0b10001,0b01010,0b00100,0b00100,0b00100,0b00100],
        'Z' => [0b11111,0b00001,0b00010,0b00100,0b01000,0b10000,0b11111],
        '0' => [0b01110,0b10001,0b10011,0b10101,0b11001,0b10001,0b01110],
        '1' => [0b00100,0b01100,0b00100,0b00100,0b00100,0b00100,0b01110],
        '2' => [0b01110,0b10001,0b00001,0b00010,0b00100,0b01000,0b11111],
        '3' => [0b11110,0b00001,0b00001,0b01110,0b00001,0b00001,0b11110],
        '4' => [0b00010,0b00110,0b01010,0b10010,0b11111,0b00010,0b00010],
        '5' => [0b11111,0b10000,0b10000,0b11110,0b00001,0b00001,0b11110],
        '6' => [0b01110,0b10000,0b10000,0b11110,0b10001,0b10001,0b01110],
        '7' => [0b11111,0b00001,0b00010,0b00100,0b01000,0b01000,0b01000],
        '8' => [0b01110,0b10001,0b10001,0b01110,0b10001,0b10001,0b01110],
        '9' => [0b01110,0b10001,0b10001,0b01111,0b00001,0b00001,0b01110],
        ':' => [0,0b00100,0b00100,0,0b00100,0b00100,0],
        '.' => [0,0,0,0,0,0b00110,0b00110],
        '/' => [0b00001,0b00010,0b00010,0b00100,0b01000,0b01000,0b10000],
        '+' => [0,0b00100,0b00100,0b11111,0b00100,0b00100,0],
        '>' => [0b10000,0b01000,0b00100,0b00010,0b00100,0b01000,0b10000],
        '-' => [0,0,0,0b11111,0,0,0],
        ' ' => [0; 7],
        _ => [0b11111,0b10001,0b00100,0b00100,0,0b00100,0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencad_ai::ExpectedEffect;
    use opencad_graph::DesignDiff;

    fn image(width: u32, height: u32, color: [u8; 4]) -> RenderImage {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..width * height {
            rgba.extend_from_slice(&color);
        }
        RenderImage {
            width,
            height,
            rgba,
            non_background_pixels: (width * height) as usize,
        }
    }

    #[test]
    fn labels_use_semantic_values_and_effect_results() {
        let diff = DesignDiff::semantic(
            "width changed",
            vec![
                SemanticChange::ParameterChanged {
                    id: "param:width".into(),
                    before: "80 mm".into(),
                    after: "100 mm".into(),
                },
                SemanticChange::MassChanged {
                    before: "76.50 g".into(),
                    after: "84.10 g".into(),
                },
            ],
        );
        let effects = vec![EffectCheck {
            effect: ExpectedEffect::ParameterExprEquals {
                id: "param:width".into(),
                expr: "100 mm".into(),
            },
            passed: true,
            message: "ok".into(),
        }];
        let labels = ReviewGifLabels::from_review(&diff, &effects);
        assert_eq!(labels.parameter_change, "WIDTH  80 MM > 100 MM");
        assert_eq!(
            labels.mass_change.as_deref(),
            Some("MASS  76.50 G > 84.10 G")
        );
        assert_eq!(labels.checks, "1/1 CHECKS PASS");
        assert!(labels.checks_passed);
    }

    #[test]
    fn comparison_is_a_five_second_four_stage_story() {
        let before = image(160, 200, [10, 20, 30, 255]);
        let after = image(160, 200, [70, 80, 90, 255]);
        let labels = ReviewGifLabels {
            before_parameter: "WIDTH  80 MM".into(),
            after_parameter: "WIDTH  100 MM".into(),
            parameter_change: "WIDTH  80 MM > 100 MM".into(),
            mass_change: Some("MASS  76.50 G > 84.10 G".into()),
            checks: "2/2 CHECKS PASS".into(),
            checks_passed: true,
        };
        let frames = comparison_frames(&before, &after, &labels, 4).expect("frames");
        assert_eq!(frames.len(), 20);
        assert_ne!(frames[0], before);
        assert_ne!(frames[8], after);
        let split = frames.last().expect("split frame");
        assert_eq!(split.rgba[((110 * 160 + 20) * 4) as usize], 10);
        assert_eq!(split.rgba[((110 * 160 + 140) * 4) as usize], 70);
    }

    #[test]
    fn comparison_rejects_incompatible_images() {
        let before = image(160, 200, [0, 0, 0, 255]);
        let after = image(161, 200, [0, 0, 0, 255]);
        let labels = ReviewGifLabels {
            before_parameter: String::new(),
            after_parameter: String::new(),
            parameter_change: String::new(),
            mass_change: None,
            checks: String::new(),
            checks_passed: true,
        };
        let error = comparison_frames(&before, &after, &labels, 8)
            .expect_err("mismatched images must fail");
        assert!(error.to_string().contains("identical pixel dimensions"));
    }
}
