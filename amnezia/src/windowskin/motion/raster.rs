use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[cfg(test)]
mod tests;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
#[require(ImageNode, Cached)]
pub(crate) struct Pixels {
    pub(crate) size: UVec2,
    pub(crate) half: u32,
}

#[derive(Component, Default)]
struct Cached(Option<Pixels>);

pub(crate) fn register(app: &mut App) {
    app.add_systems(PostUpdate, rasterize.before(bevy::ui::UiSystems::Content));
}

fn rasterize(
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut events: MessageReader<AssetEvent<Image>>,
    mut windows: Query<(&Pixels, &mut ImageNode, &mut Cached)>,
) {
    let system = server.load::<Image>("graphics/System/System.png");
    let changed = events.read().any(|event| {
        matches!(event, AssetEvent::Modified { id } | AssetEvent::LoadedWithDependencies { id } if *id == system.id())
    });
    for (pixels, mut node, mut cached) in &mut windows {
        if cached.0 == Some(*pixels) && !changed {
            continue;
        }
        let Some(skin) = images.get(&system) else {
            continue;
        };
        let image = render(skin, *pixels);
        if cached.0.is_some()
            && let Some(mut current) = images.get_mut(&node.image)
        {
            *current = image;
        } else {
            node.image = images.add(image);
        }
        cached.0 = Some(*pixels);
    }
}

pub(super) fn render(skin: &Image, pixels: Pixels) -> Image {
    let size = pixels.size.max(UVec2::splat(16));
    let half = pixels.half.min(size.y / 2);
    let top = size.y / 2 - half;
    let bottom = size.y / 2 + half;
    let mut image = Image::new_fill(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    for y in top..bottom {
        for x in 0..size.x {
            let mut color = rgba(skin, sample(x, size.x), sample(y, size.y));
            let border_y = if y < top + half.min(8) {
                Some(y - top)
            } else if y >= bottom - half.min(8) {
                Some(32 - (bottom - y))
            } else if x < 8 || x >= size.x - 8 {
                Some(8 + y % 16)
            } else {
                None
            };
            if let Some(sy) = border_y {
                let foreground = rgba(skin, 32 + tile(x, size.x), sy);
                let alpha = u32::from(foreground[3]);
                for i in 0..3 {
                    color[i] = ((u32::from(foreground[i]) * alpha
                        + u32::from(color[i]) * (255 - alpha))
                        / 255) as u8;
                }
            }
            let index = ((y * size.x + x) * 4) as usize;
            image.data.as_mut().unwrap()[index..index + 4].copy_from_slice(&color);
        }
    }
    image
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn sample(position: u32, size: u32) -> u32 {
    let scale = (32_u64 << 16) / u64::from(size);
    ((u64::from(position * 2 + 1) * scale / 2).saturating_sub(1) >> 16) as u32
}

fn tile(position: u32, size: u32) -> u32 {
    if position < 8 {
        position
    } else if position >= size - 8 {
        24 + position - (size - 8)
    } else {
        8 + position % 16
    }
}
