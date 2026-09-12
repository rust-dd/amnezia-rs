use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[cfg(test)]
mod tests;

#[derive(Component, Clone, Copy, Default, PartialEq, Eq)]
#[require(ImageNode, Cached)]
pub(super) struct WindowFrame {
    pub width: u32,
    pub opened: u32,
}

#[derive(Component, Default)]
pub(super) struct Cached(Option<WindowFrame>);

pub(super) fn rasterize(
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut events: MessageReader<AssetEvent<Image>>,
    mut frames: Query<(&WindowFrame, &mut ImageNode, &mut Cached)>,
) {
    let system = assets.load::<Image>("graphics/System/System.png");
    let changed = events.read().any(|event| {
        matches!(event, AssetEvent::Modified { id } | AssetEvent::LoadedWithDependencies { id } if *id == system.id())
    });
    for (frame, mut node, mut cache) in &mut frames {
        if cache.0 == Some(*frame) && !changed {
            continue;
        }
        let Some(skin) = images.get(&system) else {
            continue;
        };
        let rendered = render(skin, frame.width, frame.opened);
        if cache.0.is_some()
            && let Some(mut current) = images.get_mut(&node.image)
        {
            *current = rendered;
        } else {
            node.image = images.add(rendered);
        }
        node.image_mode = NodeImageMode::Stretch;
        cache.0 = Some(*frame);
    }
}

fn render(skin: &Image, width: u32, opened: u32) -> Image {
    let width = width.max(16);
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let half = opened.min(8) * 4;
    let edge = half.min(8);
    for y in 32 - half..32 + half {
        for x in 0..width {
            let background = rgba(skin, (2 * x + 1) * 16 / width, y / 2);
            let top = y - (32 - half);
            let bottom = 32 + half - y;
            let border = if top < edge {
                Some((border_x(x, width), top))
            } else if bottom <= edge {
                Some((border_x(x, width), 32 - bottom))
            } else if x < 8 || x >= width - 8 {
                Some((border_x(x, width), 8 + y % 16))
            } else {
                None
            };
            let pixel = border.map_or(background, |(sx, sy)| over(rgba(skin, sx, sy), background));
            let index = ((y * width + x) * 4) as usize;
            image.data.as_mut().unwrap()[index..index + 4].copy_from_slice(&pixel);
        }
    }
    image
}

fn border_x(x: u32, width: u32) -> u32 {
    if x < 8 {
        32 + x
    } else if x >= width - 8 {
        64 - (width - x)
    } else {
        40 + x % 16
    }
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
