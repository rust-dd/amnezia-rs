use super::{
    flash::SpriteFlash,
    tone::{self, SpriteTone},
};
use bevy::prelude::*;

#[derive(Component, Default)]
pub(super) struct Rasterized {
    source: Handle<Image>,
    color: [u8; 4],
    tone: [f32; 4],
    image: Option<Handle<Image>>,
}

pub(super) fn render(
    source: &Handle<Image>,
    flash: Option<&SpriteFlash>,
    tone: Option<&SpriteTone>,
    rasterized: &mut Rasterized,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let flash = flash.map_or([0; 4], |flash| flash.0);
    let tone = tone.map_or([100.0; 4], |tone| tone.0);
    let neutral = tone::uniform(tone) == Vec4::splat(128.0);
    if flash[3] == 0 && neutral {
        return source.clone();
    }
    if let Some(image) = &rasterized.image
        && rasterized.source == *source
        && rasterized.color == flash
        && rasterized.tone == tone
        && images.contains(image.id())
    {
        return image.clone();
    }
    let Some(mut image) = images.get(source).cloned() else {
        return source.clone();
    };
    let Some(data) = image.data.as_mut() else {
        return source.clone();
    };
    for pixel in data
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .filter(|pixel| pixel[3] != 0)
    {
        if !neutral {
            let rgb = tone::apply([pixel[0], pixel[1], pixel[2]], tone);
            pixel[..3].copy_from_slice(&rgb);
        }
        if flash[3] == 0 {
            continue;
        }
        for (channel, color) in pixel[..3].iter_mut().zip(flash) {
            let product = u32::from(*channel) * u32::from(255 - flash[3]) + 128;
            let original = (product + (product >> 8)) >> 8;
            // Pixman's solid fill stores premultiplied RGB in the high byte.
            let overlay = (u32::from(color) * u32::from(flash[3])) >> 8;
            *channel = (original + overlay).min(255) as u8;
        }
    }
    if let Some(handle) = &rasterized.image
        && let Some(mut existing) = images.get_mut(handle)
    {
        *existing = image;
    } else {
        rasterized.image = Some(images.add(image));
    }
    rasterized.source = source.clone();
    rasterized.color = flash;
    rasterized.tone = tone;
    rasterized.image.clone().unwrap()
}
