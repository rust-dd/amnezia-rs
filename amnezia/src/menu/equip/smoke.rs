use crate::audio::{AudioRequest, SystemSounds};
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::menu::{MemberAction, MenuOpen, MenuScreen, MenuState};
use crate::state::{Inventory, Party};
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Checks {
    audio: MessageCursor<AudioRequest>,
    frames: u32,
    sounds: u32,
}

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        1075 | 1080 | 1085 | 1095 | 1100 | 1125 => Some(KeyCode::Enter),
        1130 => Some(KeyCode::Space),
        1110 | 1140 | 1150 | 1160 => Some(KeyCode::Escape),
        _ => None,
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) {
    if frame == 1070 {
        world.insert_resource(Checks::default());
        world.insert_resource(Equipment::default());
        world.insert_resource(Inventory::default());
        world.resource_mut::<Inventory>().add_item(2, 1);
        world.resource_mut::<Party>().restore(vec![1, 2, 3, 4]);
        world.resource_mut::<MenuState>().screen = equip(None);
        assert!(world.resource::<MenuOpen>().0);
    }
    if (1071..=1162).contains(&frame) {
        verify_frame(world, frame);
    }
    if frame == 1084 {
        world.insert_resource(Inventory::default());
    }
    if frame == 1120 {
        world.resource_mut::<Party>().restore(vec![9]);
        world.resource_mut::<MenuState>().screen = equip(None);
        assert!(world.resource::<GameData>().actor(9).unwrap().fix_equipment);
    }
}

fn equip(picking: Option<usize>) -> MenuScreen {
    MenuScreen::Equip {
        member: 0,
        slot: 0,
        picking,
    }
}

fn verify_frame(world: &mut World, frame: u32) {
    let expected = match frame {
        1076..=1080 | 1086..=1095 | 1101..=1110 => equip(Some(0)),
        1141..=1150 => MenuScreen::MemberSelect {
            action: MemberAction::Equip,
            cursor: 0,
        },
        1151..=1162 => MenuScreen::Command,
        _ => equip(None),
    };
    assert_eq!(
        world.resource::<MenuState>().screen,
        expected,
        "equipment screen at {frame}"
    );
    assert_eq!(world.resource::<MenuOpen>().0, frame <= 1160);
    let data = world.resource::<GameData>();
    let equipment = world.resource::<Equipment>();
    let weapon = equipment.slots(data.actor(1).unwrap())[0];
    assert_eq!(
        weapon,
        if frame < 1081 {
            1
        } else if frame < 1096 {
            2
        } else {
            0
        }
    );
    assert_eq!(equipment.slots(data.actor(9).unwrap())[0], 41);
    let inventory = world.resource::<Inventory>();
    assert_eq!(
        inventory.count(1),
        u32::from((1081..=1084).contains(&frame))
    );
    assert_eq!(
        inventory.count(2),
        u32::from(!(1081..1096).contains(&frame))
    );
    if (1121..=1140).contains(&frame) {
        assert!(super::candidates(0, 0, data, world.resource::<Party>(), inventory).is_empty());
    }
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let actual = checks
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let sounds = world.resource::<SystemSounds>();
        let expected = match frame {
            1076 | 1081 | 1086 | 1096 | 1101 => Some(&sounds.decision),
            1111 | 1141 | 1151 | 1161 => Some(&sounds.cancel),
            1126 | 1131 => Some(&sounds.buzzer),
            _ => None,
        };
        let expected = expected
            .and_then(|sound| AudioRequest::se(&sound.name, sound.volume, sound.tempo))
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "equipment sounds at {frame}");
        checks.frames += 1;
        checks.sounds += actual.len() as u32;
    });
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!((checks.frames, checks.sounds), (92, 11));
    info!(
        "equipment selection: 92 states, 11 exact sounds, original Karpenge swap, empty removal, cancellation and actor 9 lock verified"
    );
}
