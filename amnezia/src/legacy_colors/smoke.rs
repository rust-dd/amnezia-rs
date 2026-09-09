use crate::picture::PictureCommand;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

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
        let camera = world
            .query_filtered::<&Transform, With<crate::world::MainCamera>>()
            .single(world)
            .unwrap()
            .translation;
        let cross = world
            .resource::<AssetServer>()
            .load(crate::assets::resolve_png("Picture", "Cross"));
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
        world.spawn((
            Sprite::from_color(Color::srgb_u8(64, 128, 192), Vec2::splat(20.0)),
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
        ("picture grayscale", 230, 15, [101, 101, 101]),
        ("picture tone", 270, 15, [16, 78, 0]),
        ("screen tone", 160, 200, [32, 64, 96]),
    ] {
        let actual = image
            .get_color_at(
                (2 * x + 1) * image.width() / 640,
                (2 * y + 1) * image.height() / 480,
            )
            .unwrap()
            .to_srgba()
            .to_u8_array();
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
