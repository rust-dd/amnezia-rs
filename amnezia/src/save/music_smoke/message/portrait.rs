use super::{command, expected, settings, start};
use crate::{
    dialogue::Dialogue,
    interpreter::RunningEvent,
    save::{LoadOutcome, LoadRequest, music_smoke::Fixture, read_save},
    teleport::Fade,
};
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Probe {
    bytes: Vec<u8>,
    checked: u8,
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        760 => {
            world.init_resource::<Probe>();
            let mut commands = settings(true);
            commands.push(command(11910, "", vec![]));
            commands.push(command(10110, "Árvíztűrő tükörfúrógép.", vec![]));
            start(world, commands);
        }
        763 => {
            let path = world.resource::<Fixture>().slot.selected_path(world);
            let saved = read_save(&path).unwrap();
            assert_eq!(saved.message, expected(true));
            assert!(saved.foreground.is_some());
            let mut probe = world.resource_mut::<Probe>();
            probe.bytes = std::fs::read(path).unwrap();
            probe.checked |= 1;
        }
        775 | 940 => {
            verify_portrait(world);
            if frame == 940 {
                verify_file(world);
            }
            world.resource_mut::<Probe>().checked |= if frame == 775 { 2 } else { 8 };
            return Some(if frame == 775 {
                "save-message-portrait-before-load"
            } else {
                "save-message-portrait"
            });
        }
        780 | 960 => world.resource_mut::<Dialogue>().close(),
        790 => {
            verify_finished_event(world);
            start(world, settings(false));
        }
        800 => {
            verify_finished_event(world);
            world.resource_mut::<LoadRequest>().0 = true;
        }
        801 => {
            assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
            assert!(world.resource::<Fade>().busy());
            assert!(!world.resource::<Dialogue>().active);
            assert!(world.resource::<RunningEvent>().active());
            assert_eq!(world.resource::<Dialogue>().face, expected(true).face);
            world.resource_mut::<Probe>().checked |= 4;
        }
        980 => {
            verify_finished_event(world);
            verify_file(world);
            world.resource_mut::<Probe>().checked |= 16;
        }
        _ => {}
    }
    None
}

fn verify_portrait(world: &mut World) {
    assert!(!world.resource::<Fade>().busy());
    assert!(world.resource::<RunningEvent>().active());
    let dialogue = world.resource::<Dialogue>();
    assert!(dialogue.active);
    assert_eq!(dialogue.face, expected(true).face);
    assert_eq!(dialogue.boxes[0].face.as_deref(), Some("Ron"));
    assert_eq!(dialogue.boxes[0].face_index, 6);
    crate::dialogue::verify_saved_presentation(world, true, true, true);
}

fn verify_finished_event(world: &World) {
    assert!(!world.resource::<RunningEvent>().active());
    assert!(!world.resource::<Dialogue>().active);
    assert_eq!(world.resource::<Dialogue>().face, default());
}

fn verify_file(world: &World) {
    let path = world.resource::<Fixture>().slot.selected_path(world);
    assert_eq!(
        std::fs::read(path).unwrap(),
        world.resource::<Probe>().bytes
    );
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Probe>().checked, 31);
}
