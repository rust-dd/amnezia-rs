use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
pub(crate) struct HueShift {
    pub original: Handle<Image>,
    pub degrees: i32,
}

#[derive(Resource, Default)]
struct Cache(HashMap<(AssetId<Image>, i32), Handle<Image>>);

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct HueSet;

pub(super) fn register(app: &mut App) {
    app.init_resource::<Cache>()
        .add_systems(PostUpdate, apply.in_set(HueSet));
}

#[allow(clippy::type_complexity)]
fn apply(
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<Cache>,
    mut sprites: Query<(
        &HueShift,
        &mut Sprite,
        Option<(&super::flash::SpriteFlash, &mut super::flash::Rasterized)>,
    )>,
) {
    for (shift, mut sprite, flash) in &mut sprites {
        let key = (shift.original.id(), shift.degrees);
        let image = if shift.degrees == 0 {
            shift.original.clone()
        } else if let Some(image) = cache.0.get(&key) {
            image.clone()
        } else {
            let Some(source) = images.get(&shift.original) else {
                continue;
            };
            let mut image = source.clone();
            let Some(data) = image.data.as_mut() else {
                continue;
            };
            assert_eq!(data.len(), (source.width() * source.height() * 4) as usize);
            for pixel in data.chunks_exact_mut(4).filter(|p| p[3] != 0) {
                let rgb = rotate([pixel[0], pixel[1], pixel[2]], shift.degrees);
                pixel[..3].copy_from_slice(&rgb);
            }
            let image = images.add(image);
            cache.0.insert(key, image.clone());
            image
        };
        let image = if let Some((flash, mut rasterized)) = flash {
            super::flash::render(&image, flash, &mut rasterized, &mut images)
        } else {
            image
        };
        if sprite.image != image {
            sprite.image = image;
        }
    }
}

pub(crate) fn rotate(rgb: [u8; 3], degrees: i32) -> [u8; 3] {
    if degrees == 0 {
        return rgb;
    }
    let [r, g, b] = rgb.map(i32::from);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let chroma = max - min;
    let sum = max + min;
    let hue = if chroma == 0 {
        0
    } else if max == r {
        256 * (g - b) / chroma + if g < b { 1536 } else { 0 }
    } else if max == g {
        256 * (b - r) / chroma + 512
    } else {
        256 * (r - g) / chroma + 1024
    };
    let saturation = if sum == 0 {
        0
    } else {
        (256 * chroma / if sum > 255 { 511 - sum } else { sum }).min(255)
    };
    let lightness = sum / 2;
    let shift = ((degrees as f64 / 60.0 * 256.0) as i64).rem_euclid(1536) as i32;
    let hue = (hue + shift) % 1536;
    let sum = lightness * 2;
    let chroma = saturation * if sum > 255 { 511 - sum } else { sum } / 256;
    let min = (sum - chroma) / 2;
    let rising = (hue & 255) * chroma / 256;
    let falling = (255 - (hue & 255)) * chroma / 256;
    let color = match hue >> 8 {
        0 => [chroma, rising, 0],
        1 => [falling, chroma, 0],
        2 => [0, chroma, rising],
        3 => [0, falling, chroma],
        4 => [rising, 0, chroma],
        _ => [chroma, 0, falling],
    };
    color.map(|v| (v + min) as u8)
}

#[cfg(test)]
mod tests;
