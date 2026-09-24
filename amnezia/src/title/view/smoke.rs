use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod pixels;

#[derive(Resource, Default)]
struct Checks {
    requested: [u8; 2],
    verified: Arc<AtomicUsize>,
}

pub(crate) struct Snapshot {
    pixels: Vec<[u8; 4]>,
    verified: Arc<AtomicUsize>,
}

pub(in crate::title) fn opening_label(world: &mut World, frame: u32) -> Option<&'static str> {
    if (60..=421).contains(&frame) || !world.resource::<TitleActive>().0 {
        return None;
    }
    let opened = world.resource::<clock::Clock>().opened;
    let index = [1, 4, 7, 8].iter().position(|&phase| phase == opened)?;
    let returning = usize::from(frame > 421);
    world.init_resource::<Checks>();
    let mut checks = world.resource_mut::<Checks>();
    if checks.requested[returning] & (1 << index) != 0 {
        return None;
    }
    checks.requested[returning] |= 1 << index;
    Some(
        [
            [
                "title-open-first",
                "title-open-half",
                "title-open-last",
                "title-open-ready",
            ],
            [
                "title-reopen-first",
                "title-reopen-half",
                "title-reopen-last",
                "title-reopen-ready",
            ],
        ][returning][index],
    )
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let expected = match label {
        "title-open-first" | "title-reopen-first" => 1,
        "title-open-half" | "title-reopen-half" => 4,
        "title-open-last" | "title-reopen-last" => 7,
        "title"
        | "title-open-ready"
        | "title-reopen-ready"
        | "title-return-ready"
        | "title-load-return"
        | "title-new-game-fade" => 8,
        label if crate::title::smoke::repetition::LABELS.contains(&label) => 8,
        _ => return None,
    };
    world.init_resource::<Checks>();
    assert!(world.resource::<TitleActive>().0);
    assert_eq!(world.resource::<clock::Clock>().opened, expected, "{label}");
    let transition = world.resource::<crate::transitions::Transition>();
    let mut pixels = pixels::expected(world, expected);
    if label == "title-new-game-fade" {
        assert!(transition.busy());
        assert_eq!(transition.age(), 1);
        for pixel in &mut pixels {
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * 128 + 127) / 255) as u8;
            }
        }
    } else {
        assert!(!transition.busy());
    }
    Some(Snapshot {
        pixels,
        verified: world.resource::<Checks>().verified.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for (index, expected) in self.pixels.iter().enumerate() {
            let (x, y) = (index as u32 % 320, index as u32 / 320);
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.abs_diff(*b) <= 1),
                "title window ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.verified.fetch_add(1, Ordering::Relaxed);
        info!(
            "title window: {} original background, opening, cursor and glyph pixels verified",
            self.pixels.len()
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!(checks.requested, [15, 15]);
    assert_eq!(checks.verified.load(Ordering::Relaxed), 14);
}

pub(crate) fn verify_load_finished(world: &World) {
    assert_eq!(
        world.resource::<Checks>().verified.load(Ordering::Relaxed),
        2
    );
}

pub(in crate::title) fn verify_new_game_finished(world: &World) {
    assert_eq!(
        world.resource::<Checks>().verified.load(Ordering::Relaxed),
        2
    );
}
