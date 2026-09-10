use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const CASES: [(bool, u32, u16, u16); 8] = [
    (false, 0, 0, 0),
    (false, 6, 0, 1),
    (false, 12, 0, 2),
    (false, 18, 0, 3),
    (false, 24, 1, 0),
    (true, 12, 1, 2),
    (true, 24, 2, 0),
    (true, 36, 1, 2),
];
const LABELS: [&str; 8] = [
    "water-slow-0",
    "water-slow-6",
    "water-slow-12",
    "water-slow-18",
    "water-slow-24",
    "water-fast-12",
    "water-fast-24",
    "water-fast-36",
];

#[derive(Resource)]
struct Fixture {
    image: Handle<Image>,
    complete: Arc<AtomicUsize>,
}

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 3])>,
    complete: Arc<AtomicUsize>,
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        world.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
        let image = world
            .resource::<AssetServer>()
            .load(crate::assets::resolve_png("ChipSet", "basis"));
        let camera = world
            .query_filtered::<&Transform, With<crate::world::MainCamera>>()
            .single(world)
            .unwrap()
            .translation;
        world.spawn((
            Sprite::from_color(Color::BLACK, Vec2::new(96.0, 48.0)),
            Transform::from_xyz(camera.x, camera.y, 899.0),
        ));
        for quarter in 0..4 {
            let x = (quarter % 2) as f32 * 8.0;
            let y = (quarter / 2) as f32 * 8.0;
            world.spawn((
                WaterQuarter { id: 0, quarter },
                Sprite::from_image(image.clone()),
                Transform::from_xyz(camera.x - 36.0 + x, camera.y + 4.0 - y, 900.0),
            ));
        }
        world.spawn((
            WaterCell { id: 3000 },
            Sprite::from_image(image.clone()),
            Transform::from_xyz(camera.x + 32.0, camera.y, 900.0),
        ));
        world.insert_resource(Fixture {
            image,
            complete: Arc::new(AtomicUsize::new(0)),
        });
    }
    if (260..=960).contains(&frame) && (frame - 260).is_multiple_of(100) {
        let (fast, time, _, _) = CASES[((frame - 260) / 100) as usize];
        world.insert_resource(WaterStyle { fast, cycle: false });
        world.resource_mut::<GameFrames>().frame = time;
    }
    if (290..=990).contains(&frame) && (frame - 290).is_multiple_of(100) {
        Some(LABELS[((frame - 290) / 100) as usize])
    } else {
        None
    }
}

pub(crate) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    let index = LABELS.iter().position(|s| *s == label)?;
    let fixture = world.resource::<Fixture>();
    let source = world
        .resource::<Assets<Image>>()
        .get(&fixture.image)
        .unwrap();
    let (_, _, ab, c) = CASES[index];
    let mut pixels = Vec::new();
    for y in 0..16 {
        for x in 0..16 {
            for (left, sx, sy) in [
                (120, ab as u32 * 16 + x, 64 + y),
                (184, 48 + x, 64 + c as u32 * 16 + y),
            ] {
                let color = source
                    .get_color_at(sx, sy)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                pixels.push((
                    left + x,
                    112 + y,
                    if color[3] == 0 {
                        [0; 3]
                    } else {
                        [color[0], color[1], color[2]]
                    },
                ));
            }
        }
    }
    Some(Snapshot {
        pixels,
        complete: fixture.complete.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image, label: &str) {
        for &(x, y, expected) in &self.pixels {
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
                "{label} ({x},{y}): {actual:?}, expected {expected:?}"
            );
        }
        self.complete.fetch_add(1, Ordering::SeqCst);
        info!("{label}: all 512 water and waterfall pixels match the original animation frames");
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(
        world.resource::<Fixture>().complete.load(Ordering::SeqCst),
        CASES.len()
    );
}
