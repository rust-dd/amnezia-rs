use super::*;
use crate::dialogue::{MessageOptions, MessagePosition, saved::MessageState};
use crate::events::{MessageFace, message_boxes};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

#[derive(Resource, Default)]
struct Probe {
    checked: u8,
    pixels: Arc<AtomicU32>,
}

fn command(code: u32, text: &str, params: Vec<i32>) -> amnezia_data::EventCommand {
    amnezia_data::EventCommand {
        code,
        indent: 0,
        string: text.into(),
        params,
    }
}

fn settings(portrait: bool) -> Vec<amnezia_data::EventCommand> {
    vec![
        command(
            10120,
            "",
            if portrait {
                vec![1, 0, 0, 1]
            } else {
                vec![0, 2, 0, 0]
            },
        ),
        command(
            10130,
            if portrait { "Ron" } else { "" },
            vec![if portrait { 6 } else { 0 }, 0, 0],
        ),
    ]
}

fn expected(portrait: bool) -> MessageState {
    let mut face = MessageFace::default();
    message_boxes(&settings(portrait), &mut face);
    MessageState {
        position: if portrait {
            MessagePosition::Top
        } else {
            MessagePosition::Bottom
        },
        transparent: portrait,
        options: MessageOptions {
            fixed: true,
            continue_events: portrait,
        },
        face,
    }
}

fn start(world: &mut World, commands: Vec<amnezia_data::EventCommand>) {
    let mut running = world.resource_mut::<RunningEvent>();
    assert!(!running.active());
    running.start(0, commands);
}

fn show(world: &mut World, expected: MessageState) {
    let actual = world
        .run_system_once(
            |capture: crate::dialogue::saved::Capture, dialogue: Res<Dialogue>| {
                capture.snapshot(&dialogue)
            },
        )
        .unwrap();
    assert_eq!(actual, expected);
    assert!(!world.resource::<Dialogue>().active);
    start(
        world,
        vec![command(10110, "Árvíztűrő tükörfúrógép.", vec![])],
    );
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        270 => {
            world.init_resource::<Probe>();
            start(world, settings(true));
        }
        303 | 493 => {
            let path = world.resource::<Fixture>().slot.path(world);
            assert_eq!(read_save(&path).unwrap().message, expected(frame == 303));
        }
        310 | 480 => start(world, settings(false)),
        495 => start(world, settings(true)),
        410 => show(world, expected(true)),
        590 => show(world, expected(false)),
        700 => show(world, MessageState::default()),
        445 | 604 | 735 => {
            let portrait = frame == 445;
            crate::dialogue::verify_saved_presentation(world, frame != 604, portrait, portrait);
            let dialogue = world.resource::<Dialogue>();
            assert!(dialogue.active);
            assert_eq!(dialogue.boxes[0].face.as_deref(), portrait.then_some("Ron"));
            if portrait {
                assert_eq!(dialogue.boxes[0].face_index, 6);
            }
            world.resource_mut::<Probe>().checked |= match frame {
                445 => 1,
                604 => 2,
                _ => 4,
            };
            return Some(match frame {
                445 => "save-message-portrait",
                604 => "save-message-cleared",
                _ => "save-message-legacy",
            });
        }
        455 | 606 | 745 => world.resource_mut::<Dialogue>().close(),
        _ => {}
    }
    None
}

pub(crate) struct Snapshot {
    portrait: Image,
    verified: Arc<AtomicU32>,
}

pub(crate) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    if label != "save-message-portrait" {
        return None;
    }
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("FaceSet", "Ron"));
    Some(Snapshot {
        portrait: world
            .resource::<Assets<Image>>()
            .get(&handle)
            .unwrap()
            .clone(),
        verified: world.resource::<Probe>().pixels.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        let mut count = 0;
        for y in 0..48 {
            for x in 0..48 {
                let expected = self
                    .portrait
                    .get_color_at(96 + x, 48 + y)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                if expected[3] != 255 {
                    continue;
                }
                let actual = crate::display::smoke::pixel_at(image, 16 + x, 16 + y);
                for channel in 0..3 {
                    assert!(
                        actual[channel].abs_diff(expected[channel]) <= 1,
                        "restored portrait ({x},{y}): expected {expected:?}, got {actual:?}"
                    );
                }
                count += 1;
            }
        }
        assert!(count > 500);
        self.verified.fetch_add(1, Ordering::SeqCst);
        info!("saved dialogue: {count} original portrait pixels verified");
    }
}

pub(super) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.checked, 7);
    assert_eq!(probe.pixels.load(Ordering::SeqCst), 1);
    info!(
        "saved dialogue: portrait, cleared face, placement, transparency and legacy defaults verified"
    );
}
