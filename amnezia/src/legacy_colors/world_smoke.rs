use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

const TONES: [[f32; 4]; 6] = [
    [100.0; 4],
    [200.0, 200.0, 200.0, 100.0],
    [50.0, 100.0, 150.0, 0.0],
    [100.0, 100.0, 100.0, 150.0],
    [0.0, 0.0, 0.0, 100.0],
    [100.0; 4],
];
const LABELS: [&str; 6] = [
    "world-tone-neutral",
    "world-tone-white",
    "world-tone-gray",
    "world-tone-saturated",
    "world-tone-black",
    "world-tone-restored",
];
const RGB: [u8; 3] = [32, 156, 0];

#[derive(Resource, Default)]
struct Verified(std::sync::Arc<std::sync::atomic::AtomicUsize>);

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        world.init_resource::<Verified>();
        let scene = world
            .query_filtered::<Entity, With<crate::world::MapScene>>()
            .iter(world)
            .collect::<Vec<_>>();
        for entity in scene {
            world.despawn(entity);
        }
        world.resource_mut::<crate::player::HeroHidden>().0 = true;
        let camera = world
            .query_filtered::<&Transform, With<crate::world::MainCamera>>()
            .single(world)
            .unwrap()
            .translation;
        for (x, alpha, opacity, layer) in [
            (60.0, 255, 1.0, 0),
            (100.0, 255, 0.4, 0),
            (140.0, 128, 0.4, 0),
            (220.0, 255, 1.0, crate::screenfx::PICTURE_LAYER),
        ] {
            let image = Image::new_fill(
                bevy::render::render_resource::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                &[RGB[0], RGB[1], RGB[2], alpha],
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::default(),
            );
            let image = world.resource_mut::<Assets<Image>>().add(image);
            world.spawn((
                Sprite {
                    image,
                    color: Color::WHITE.with_alpha(opacity),
                    custom_size: Some(Vec2::splat(24.0)),
                    ..default()
                },
                Transform::from_xyz(camera.x + x - 160.0, camera.y + 40.0, 100.0),
                RenderLayers::layer(layer),
            ));
        }
    }
    if (260..=960).contains(&frame) && (frame - 260).is_multiple_of(140) {
        world
            .resource_mut::<crate::screenfx::TintState>()
            .set_tone(TONES[((frame - 260) / 140) as usize]);
    }
    ((320..=1020).contains(&frame) && (frame - 320).is_multiple_of(140))
        .then(|| LABELS[((frame - 320) / 140) as usize])
}

pub(crate) struct Snapshot {
    tone: [f32; 4],
    complete: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

pub(crate) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    let index = LABELS.iter().position(|&value| value == label)?;
    Some(Snapshot {
        tone: TONES[index],
        complete: world.resource::<Verified>().0.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        let toned = super::tone::apply(RGB, self.tone);
        for (x, y, expected) in [
            (20, 20, [0; 3]),
            (60, 80, toned),
            (
                100,
                80,
                toned.map(|c| ((u32::from(c) * 102 + 127) / 255) as u8),
            ),
            (
                140,
                80,
                toned.map(|c| ((u32::from(c) * 51 + 127) / 255) as u8),
            ),
            (220, 80, RGB),
        ] {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual[..3]
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                "world tone {:?} at ({x},{y}): {actual:?}, expected {expected:?}",
                self.tone
            );
        }
        self.complete
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        info!("world tone: clear pixels, texture/sprite opacity and untinted foreground verified");
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(
        world
            .resource::<Verified>()
            .0
            .load(std::sync::atomic::Ordering::Relaxed),
        LABELS.len()
    );
}
