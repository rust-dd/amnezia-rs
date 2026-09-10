use bevy::prelude::*;

#[derive(Component, Clone, Copy, Default, PartialEq, Eq)]
#[require(Rasterized)]
pub(crate) struct SpriteFlash(pub [u8; 4]);

#[derive(Component, Default)]
pub(super) struct Rasterized {
    source: Handle<Image>,
    color: [u8; 4],
    image: Handle<Image>,
}

pub(super) fn render(
    source: &Handle<Image>,
    flash: &SpriteFlash,
    rasterized: &mut Rasterized,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    if flash.0[3] == 0 {
        return source.clone();
    }
    if rasterized.source == *source
        && rasterized.color == flash.0
        && images.contains(rasterized.image.id())
    {
        return rasterized.image.clone();
    }
    let Some(mut image) = images.get(source).cloned() else {
        return source.clone();
    };
    let Some(data) = image.data.as_mut() else {
        return source.clone();
    };
    for pixel in data.chunks_exact_mut(4).filter(|pixel| pixel[3] != 0) {
        for (channel, color) in pixel[..3].iter_mut().zip(flash.0) {
            let product = u32::from(*channel) * u32::from(255 - flash.0[3]) + 128;
            let original = (product + (product >> 8)) >> 8;
            // Pixman's solid fill stores premultiplied RGB in the high byte.
            let overlay = (u32::from(color) * u32::from(flash.0[3])) >> 8;
            *channel = (original + overlay).min(255) as u8;
        }
    }
    if let Some(mut existing) = images.get_mut(&rasterized.image) {
        *existing = image;
    } else {
        rasterized.image = images.add(image);
    }
    rasterized.source = source.clone();
    rasterized.color = flash.0;
    rasterized.image.clone()
}

#[cfg(test)]
mod tests;
