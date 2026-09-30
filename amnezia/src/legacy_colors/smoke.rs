use crate::picture::PictureCommand;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

const PICTURE_SAMPLES: [(&str, u32, u32, [u8; 3]); 5] = [
    ("picture grayscale", 234, 19, [101, 101, 101]),
    ("picture tone", 274, 19, [16, 78, 0]),
    ("saturation before color tone", 34, 94, [50, 101, 179]),
    ("original oversaturation", 74, 94, [0, 211, 0]),
    ("hard-light brightening", 114, 94, [145, 207, 129]),
];

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        world.write_message(PictureCommand::show(1, "Staff1", 74.0, 120.0, &[]));
        world.write_message(PictureCommand::show(
            2,
            "Cross",
            40.0,
            25.0,
            &[0, 0, 0, 0, 0, 800, 60, 1, 100, 100, 100, 100, 0, 0],
        ));
        world.write_message(PictureCommand::show(
            3,
            "Cross",
            240.0,
            25.0,
            &[0, 0, 0, 0, 0, 800, 0, 0, 100, 100, 100, 0, 0, 0],
        ));
        world.write_message(PictureCommand::show(
            4,
            "Cross",
            280.0,
            25.0,
            &[0, 0, 0, 0, 0, 800, 0, 0, 50, 50, 50, 100, 0, 0],
        ));
        for (id, x, rgb, saturation) in [
            (5, 40.0, [50, 100, 150], 0),
            (6, 80.0, [100, 100, 100], 150),
            (7, 120.0, [150, 150, 150], 100),
        ] {
            world.write_message(PictureCommand::show(
                id,
                "Cross",
                x,
                100.0,
                &[
                    0, 0, 0, 0, 0, 800, 0, 0, rgb[0], rgb[1], rgb[2], saturation, 0, 0,
                ],
            ));
        }
        let camera = world
            .query_filtered::<&Transform, With<crate::world::MainCamera>>()
            .single(world)
            .unwrap()
            .translation;
        let cross = world
            .resource::<AssetServer>()
            .load(crate::assets::resolve_png("Picture", "Cross"));
        let source = Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            &[32, 156, 0, 255],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        );
        let source = world.resource_mut::<Assets<Image>>().add(source);
        for (x, degrees) in [(200.0, 120), (240.0, 330)] {
            world.spawn((
                Sprite::from_image(source.clone()),
                super::hue::HueShift {
                    original: source.clone(),
                    degrees,
                },
                Transform::from_xyz(camera.x + x - 160.0, camera.y + 20.0, 106.0)
                    .with_scale(Vec3::splat(20.0)),
                RenderLayers::layer(crate::screenfx::PICTURE_LAYER),
            ));
        }
        world.spawn((
            Sprite {
                image: cross,
                color: Color::WHITE.with_alpha(0.4),
                ..default()
            },
            Transform::from_xyz(camera.x - 80.0, camera.y + 95.0, 105.0)
                .with_scale(Vec3::splat(8.0)),
            RenderLayers::layer(crate::screenfx::PICTURE_LAYER),
        ));
        for (x, y, size, color) in [
            (108.0, 13.0, 24.0, Color::WHITE.with_alpha(0.4)),
            (114.0, 19.0, 12.0, Color::WHITE.with_alpha(0.4)),
            (188.0, 13.0, 24.0, Color::srgb_u8(64, 128, 192)),
            (188.0, 13.0, 24.0, Color::srgba_u8(192, 64, 128, 102)),
        ] {
            world.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(x * 3.0),
                    top: px(y * 3.0),
                    width: px(size * 3.0),
                    height: px(size * 3.0),
                    ..default()
                },
                BackgroundColor(color),
                GlobalZIndex(999),
            ));
        }
        let swatch = Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            &[64, 128, 192, 255],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        );
        let swatch = world.resource_mut::<Assets<Image>>().add(swatch);
        world.spawn((
            Sprite {
                image: swatch,
                custom_size: Some(Vec2::splat(20.0)),
                ..default()
            },
            Transform::from_xyz(camera.x, camera.y - 80.0, 10.0),
        ));
        world
            .resource_mut::<crate::screenfx::TintState>()
            .set_tone([50.0, 50.0, 50.0, 100.0]);
    }
    (frame == 320).then_some("legacy-colors")
}

pub(crate) fn verify(image: &Image, label: &str) {
    if label != "legacy-colors" {
        return;
    }
    for (name, x, y, expected) in [
        ("picture alpha", 40, 25, [102, 102, 102]),
        ("sprite alpha", 80, 25, [102, 102, 102]),
        ("UI alpha", 110, 15, [102, 102, 102]),
        ("stacked UI alpha", 120, 25, [163, 163, 163]),
        ("colored UI alpha", 200, 25, [115, 102, 166]),
        ("screen tone", 160, 200, [32, 64, 96]),
        ("120 degree hue", 200, 100, [0, 30, 155]),
        ("330 degree hue", 240, 100, [108, 155, 0]),
    ]
    .into_iter()
    .chain(PICTURE_SAMPLES)
    {
        let actual = crate::display::smoke::pixel_at(image, x, y);
        assert!(
            actual[..3]
                .iter()
                .zip(expected)
                .all(|(a, b)| a.abs_diff(b) <= 1),
            "{name} at ({x}, {y}): {actual:?}, expected {expected:?}"
        );
    }
    info!("legacy picture, sprite, UI, stacked alpha and tone GPU checks passed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_samples_use_the_original_odd_picture_origin() {
        let bytes = std::fs::read(format!(
            "{}/graphics/Picture/Cross.png",
            crate::assets::asset_root()
        ))
        .unwrap();
        let source = Image::from_buffer(
            &bytes,
            bevy::image::ImageType::Extension("png"),
            bevy::image::CompressedImageFormats::NONE,
            true,
            bevy::image::ImageSampler::nearest(),
            default(),
        )
        .unwrap();
        assert_eq!(source.size(), UVec2::splat(3));
        let zoom = 8;
        for ((_, x, y, _), (cx, cy)) in PICTURE_SAMPLES.into_iter().zip([
            (240, 25),
            (280, 25),
            (40, 100),
            (80, 100),
            (120, 100),
        ]) {
            let left = cx - source.size().x / 2 * zoom;
            let top = cy - source.size().y / 2 * zoom;
            assert!(x >= left && y >= top);
            let texel = ((x - left) / zoom, (y - top) / zoom);
            assert_eq!(texel, (0, 0));
            assert_eq!(
                source
                    .get_color_at(texel.0, texel.1)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array()[..3],
                [32, 156, 0]
            );
            assert!(x - 4 < left && y - 4 < top);
        }
    }
}
