use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
#[require(ImageNode, Cached)]
pub(super) struct Pixels(pub UVec2);

#[derive(Component, Default)]
struct Cached(Option<UVec2>);

pub(crate) fn register(app: &mut App) {
    app.add_systems(PostUpdate, rasterize.before(bevy::ui::UiSystems::Content));
}

fn rasterize(
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut events: MessageReader<AssetEvent<Image>>,
    mut backgrounds: Query<(&Pixels, &mut ImageNode, &mut Cached)>,
) {
    let system = server.load::<Image>("graphics/System/System.png");
    let changed = events.read().any(|event| {
        matches!(event, AssetEvent::Modified { id } | AssetEvent::LoadedWithDependencies { id } if *id == system.id())
    });
    for (pixels, mut node, mut cached) in &mut backgrounds {
        if cached.0 == Some(pixels.0) && !changed {
            continue;
        }
        let Some(skin) = images.get(&system) else {
            continue;
        };
        let image = render(skin, pixels.0);
        if cached.0.is_some()
            && let Some(mut current) = images.get_mut(&node.image)
        {
            *current = image;
        } else {
            node.image = images.add(image);
        }
        node.image_mode = NodeImageMode::Stretch;
        cached.0 = Some(pixels.0);
    }
}

fn render(skin: &Image, size: UVec2) -> Image {
    let size = size.max(UVec2::ONE);
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
    let data = image.data.as_mut().unwrap();
    for y in 0..size.y {
        for x in 0..size.x {
            let color = skin
                .get_color_at(sample(x, size.x), sample(y, size.y))
                .unwrap()
                .to_srgba()
                .to_u8_array();
            let index = ((y * size.x + x) * 4) as usize;
            data[index..index + 4].copy_from_slice(&color);
        }
    }
    image
}

fn sample(position: u32, length: u32) -> u32 {
    // Pixman's nearest sampling uses a truncated 16.16 scale and lower texel ties.
    let scale = (32_u64 << 16) / u64::from(length);
    ((u64::from(position * 2 + 1) * scale / 2).saturating_sub(1) >> 16).min(31) as u32
}

#[cfg(test)]
mod tests;
