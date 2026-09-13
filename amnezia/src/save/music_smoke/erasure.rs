use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

#[derive(Resource, Default)]
struct Probe {
    slot_bytes: Vec<u8>,
    checks: u8,
    pixels: Arc<AtomicU32>,
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        610 => {
            let path = world.resource::<Fixture>().slot.path(world);
            world.insert_resource(Probe {
                slot_bytes: std::fs::read(path).unwrap(),
                ..default()
            });
            let mut transition = world.resource_mut::<crate::transitions::Transition>();
            transition.hold_black();
            transition.event_erased = true;
        }
        611 => {
            assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
            assert!(world.resource::<Fade>().busy());
            assert!(
                !world
                    .resource::<crate::transitions::Transition>()
                    .event_erased
            );
            world.resource_mut::<Probe>().checks |= 1;
            return Some("save-erased-load-black");
        }
        699 => {
            assert!(!world.resource::<Fade>().busy());
            let transition = world.resource::<crate::transitions::Transition>();
            assert!(!transition.event_erased && !transition.erased());
            assert!(!world.resource::<RunningEvent>().active());
            let path = world.resource::<Fixture>().slot.path(world);
            assert_eq!(
                std::fs::read(path).unwrap(),
                world.resource::<Probe>().slot_bytes
            );
            world.resource_mut::<Probe>().checks |= 2;
            return Some("save-erased-load-visible");
        }
        _ => {}
    }
    None
}

pub(crate) struct Snapshot {
    black: bool,
    pixels: Arc<AtomicU32>,
}

pub(crate) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    let black = match label {
        "save-erased-load-black" => true,
        "save-erased-load-visible" => false,
        _ => return None,
    };
    Some(Snapshot {
        black,
        pixels: world.resource::<Probe>().pixels.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        let mut visible = 0;
        for y in 0..if self.black { 240 } else { 160 } {
            for x in 0..320 {
                let pixel = crate::display::smoke::pixel_at(image, x, y);
                if self.black {
                    assert_eq!(&pixel[..3], &[0, 0, 0], "erased load ({x},{y})");
                } else if pixel[..3].iter().any(|&channel| channel > 16) {
                    visible += 1;
                }
            }
        }
        if !self.black {
            assert!(
                visible > 10_000,
                "the restored map must be visible, got {visible} pixels"
            );
        }
        self.pixels
            .fetch_or(if self.black { 1 } else { 2 }, Ordering::Relaxed);
        info!(
            "saved erasure: black={}, visible map pixels={visible}",
            self.black
        );
    }
}

pub(super) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.checks, 3);
    assert_eq!(probe.pixels.load(Ordering::Relaxed), 3);
}
