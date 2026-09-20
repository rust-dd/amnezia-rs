use super::Case;
use bevy::prelude::*;

pub(super) fn reference(
    skin: &Image,
    glyphs: &Image,
    portrait: Option<&Image>,
    case: &Case,
    index: usize,
    origin: u32,
) -> Vec<[u8; 4]> {
    let mut pixels = vec![[0, 0, 0, 255]; 320 * 240];
    if !case.transparent {
        for y in 0..80 {
            for x in 0..320 {
                let mut pixel = rgba(skin, background(x, 320), background(y, 80));
                if !(8..312).contains(&x) || !(8..72).contains(&y) {
                    pixel = over(
                        rgba(skin, 32 + coordinate(x, 320), coordinate(y, 80)),
                        pixel,
                    );
                }
                pixels[((case.top + y) * 320 + x) as usize] = pixel;
            }
        }
    }
    let offset = if case.face { 72 } else { 0 };
    let (left, top, width) = if case.digits == 0 {
        (
            10 + offset,
            8 + if index == 2 { 32 } else { 48 },
            300 - offset,
        )
    } else {
        (16 + offset + (case.digits - 1) * 12, 8, 14)
    };
    let cursor = cursor(skin, width, origin);
    for y in 0..16 {
        for x in 0..width {
            let pixel = &mut pixels[((case.top + top + y) * 320 + left + x) as usize];
            *pixel = over(cursor[(y * width + x) as usize], *pixel);
        }
    }
    if let Some(portrait) = portrait {
        for y in 0..48 {
            for x in 0..48 {
                let pixel = &mut pixels[((case.top + 16 + y) * 320 + 16 + x) as usize];
                *pixel = over(rgba(portrait, 96 + x, 48 + y), *pixel);
            }
        }
    }
    for y in 0..64 {
        for x in 0..304 {
            let pixel = &mut pixels[((case.top + 8 + y) * 320 + 8 + x) as usize];
            *pixel = over(rgba(glyphs, x, y), *pixel);
        }
    }
    pixels
}

fn cursor(skin: &Image, width: u32, origin: u32) -> Vec<[u8; 4]> {
    let mut pixels = vec![[0; 4]; width as usize * 16];
    for y in 0..16 {
        for x in 8..width.saturating_sub(8) {
            pixels[(y * width + x) as usize] = rgba(
                skin,
                origin + 8 + (x - 8) % 16,
                if y < 8 { y } else { y + 16 },
            );
        }
    }
    for (left, top, sx, sy) in [
        (0, 0, 0, 0),
        (width - 8, 0, 24, 0),
        (0, 8, 0, 24),
        (width - 8, 8, 24, 24),
    ] {
        for y in 0..8 {
            for x in 0..8 {
                let pixel = rgba(skin, origin + sx + x, sy + y);
                assert!(pixel[3] == 0 || pixel[3] == 255);
                if pixel[3] != 0 {
                    pixels[((top + y) * width + left + x) as usize] = pixel;
                }
            }
        }
    }
    pixels
}

fn coordinate(position: u32, length: u32) -> u32 {
    if position < 8 {
        position
    } else if position >= length - 8 {
        24 + position - (length - 8)
    } else {
        8 + (position - 8) % 16
    }
}

fn background(position: u32, length: u32) -> u32 {
    let scale = (32 << 16) / length;
    ((2 * position + 1) * scale / 2).saturating_sub(1) >> 16
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn over(foreground: [u8; 4], background: [u8; 4]) -> [u8; 4] {
    let alpha = u32::from(foreground[3]);
    let mut result = [0, 0, 0, 255];
    for i in 0..3 {
        result[i] = ((u32::from(foreground[i]) * alpha + u32::from(background[i]) * (255 - alpha))
            / 255) as u8;
    }
    result
}
