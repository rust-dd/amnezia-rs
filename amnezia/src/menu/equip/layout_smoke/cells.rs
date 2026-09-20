use super::LONG_NAME;
use crate::audio::{AudioRequest, SystemSounds};
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::menu::{MenuOpen, MenuScreen, MenuState};
use crate::state::Inventory;
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Checks {
    audio: MessageCursor<AudioRequest>,
    frames: u32,
    sounds: u32,
}

pub(super) fn input(frame: u32) -> Option<KeyCode> {
    matches!(frame, 1230 | 1240 | 1250).then_some(KeyCode::Escape)
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1170 {
        assert!(!world.resource::<MenuOpen>().0);
        world.insert_resource(Checks::default());
        world.resource_mut::<MenuOpen>().0 = true;
        world.resource_mut::<MenuState>().screen = MenuScreen::Equip {
            member: 0,
            slot: 0,
            picking: Some(0),
        };
        world
            .resource_mut::<GameData>()
            .items
            .iter_mut()
            .find(|item| item.id == 1)
            .unwrap()
            .name = LONG_NAME.into();
    }
    if (1171..=1252).contains(&frame) {
        verify_frame(world, frame);
    }
    if frame == 1190 {
        let mut inventory = world.resource_mut::<Inventory>();
        inventory.remove_item(2, 1);
        inventory.remove_item(3, 1);
    }
    if frame == 1210 {
        world.resource_mut::<Inventory>().remove_item(1, 1);
    }
    match frame {
        1180 => Some("equipment-cell-overlap"),
        1205 => Some("equipment-cell-empty"),
        1220 => Some("equipment-cell-cleared"),
        _ => None,
    }
}

fn verify_frame(world: &mut World, frame: u32) {
    let expected = if frame <= 1240 {
        MenuScreen::Equip {
            member: 0,
            slot: 0,
            picking: (frame <= 1230).then_some(0),
        }
    } else {
        MenuScreen::Command
    };
    assert_eq!(world.resource::<MenuState>().screen, expected);
    assert_eq!(world.resource::<MenuOpen>().0, frame <= 1250);
    let data = world.resource::<GameData>();
    assert_eq!(
        world.resource::<Equipment>().slots(data.actor(1).unwrap()),
        [2, 0, 64, 83, 0]
    );
    let inventory = world.resource::<Inventory>();
    assert_eq!(inventory.count(1), u32::from(frame <= 1210));
    for id in [2, 3] {
        assert_eq!(inventory.count(id), u32::from(frame <= 1190));
    }
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let actual = checks
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let expected = if matches!(frame, 1231 | 1241 | 1251) {
            let sound = &world.resource::<SystemSounds>().cancel;
            AudioRequest::se(&sound.name, sound.volume, sound.tempo)
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        assert_eq!(actual, expected, "list cell sounds at {frame}");
        checks.frames += 1;
        checks.sounds += actual.len() as u32;
    });
}

pub(super) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!((checks.frames, checks.sounds), (82, 3));
    info!("equipment list cells: 82 states and three exact sounds verified");
}
