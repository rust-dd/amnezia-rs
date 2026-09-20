use super::Case;
use bevy::prelude::*;

pub(super) fn reference(
    skin: &Image,
    glyphs: &Image,
    portrait: Option<&Image>,
    case: &Case,
    arrow: bool,
) -> Vec<(u32, u32, [u8; 4])> {
    let mut result = Vec::new();
    for y in 0..80 {
        for x in 0..320 {
            let mut pixel = if case.transparent {
                (!(4..316).contains(&x)).then_some([0, 0, 0, 255])
            } else {
                let background = rgba(skin, background(x, 320), background(y, 80));
                let border = !(8..312).contains(&x) || !(8..72).contains(&y);
                Some(if border {
                    over(
                        rgba(skin, 32 + coordinate(x, 320), coordinate(y, 80)),
                        background,
                    )
                } else {
                    background
                })
            };
            if let Some(portrait) = portrait
                && (16..64).contains(&x)
                && (16..64).contains(&y)
            {
                pixel = overlay(rgba(portrait, x - 16, y - 16), pixel);
            }
            if (8..312).contains(&x) && (8..72).contains(&y) {
                pixel = overlay(rgba(glyphs, x - 8, y - 8), pixel);
            }
            if arrow && (152..168).contains(&x) && y >= 72 {
                pixel = overlay(rgba(skin, 40 + x - 152, 16 + y - 72), pixel);
            }
            if let Some(pixel) = pixel {
                result.push((x, case.top + y, pixel));
            }
        }
    }
    result
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

fn overlay(foreground: [u8; 4], background: Option<[u8; 4]>) -> Option<[u8; 4]> {
    match foreground[3] {
        0 => background,
        255 => Some(foreground),
        _ => background.map(|background| over(foreground, background)),
    }
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
