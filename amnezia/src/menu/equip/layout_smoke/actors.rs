use super::{ARMOR, Checks as LayoutChecks};
use crate::audio::{AudioRequest, SystemSounds};
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::menu::{MenuOpen, MenuScreen, MenuState, equip::Scene};
use crate::state::{Inventory, Party};
use crate::text::HeroName;
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Checks {
    audio: MessageCursor<AudioRequest>,
    frames: u32,
    sounds: u32,
}

pub(super) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        730 | 900 | 970 => Some(KeyCode::Enter),
        905 => Some(KeyCode::Space),
        740 | 890 | 910 | 950 | 960 | 980 | 1040 => Some(KeyCode::ArrowRight),
        920 | 930 | 1050 => Some(KeyCode::ArrowLeft),
        750 | 760 | 770 | 780 | 790 | 800 | 810 | 820 => Some(KeyCode::ArrowDown),
        830 | 990 | 1010 | 1020 | 1060 | 1070 => Some(KeyCode::Escape),
        940 => Some(KeyCode::PageUp),
        _ => None,
    }
}

pub(crate) fn held_input(world: &mut World, frame: u32) -> bool {
    if !(840..880).contains(&frame) && frame != 1000 {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    if matches!(frame, 840 | 1000) {
        keys.reset_all();
    }
    keys.press(KeyCode::ArrowRight);
    if frame == 1000 {
        keys.press(KeyCode::ArrowDown);
    }
    true
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 720 {
        assert!(!world.resource::<MenuOpen>().0);
        let original = world
            .resource::<LayoutChecks>()
            .original_item
            .clone()
            .unwrap();
        *world
            .resource_mut::<GameData>()
            .items
            .iter_mut()
            .find(|item| item.id == 2)
            .unwrap() = original;
        world.insert_resource(Checks::default());
        world.resource_mut::<HeroName>().0 = "Áron".into();
        world.resource_mut::<Party>().restore(vec![1, 2, 9]);
        world.resource_mut::<MenuOpen>().0 = true;
        world.resource_mut::<MenuState>().screen = screen(0, 2, None);
    }
    if (721..=1072).contains(&frame) {
        verify_frame(world, frame);
    }
    if frame == 1030 {
        assert!(!world.resource::<MenuOpen>().0);
        world.resource_mut::<Party>().restore(vec![1]);
        world.resource_mut::<MenuOpen>().0 = true;
        world.resource_mut::<MenuState>().screen = screen(0, 0, None);
    }
    match frame {
        805 => Some("equipment-switch-scrolled"),
        845 => Some("equipment-switch-tiffany"),
        875 => Some("equipment-switch-held"),
        895 => Some("equipment-switch-fixed"),
        915 => Some("equipment-switch-wrapped"),
        945 => Some("equipment-switch-tiffany-weapon"),
        985 => Some("equipment-switch-picker"),
        1005 => Some("equipment-switch-shield"),
        _ => None,
    }
}

fn screen(member: usize, slot: usize, picking: Option<usize>) -> MenuScreen {
    MenuScreen::Equip {
        member,
        slot,
        picking,
    }
}

fn expected(frame: u32) -> MenuScreen {
    match frame {
        721..=730 | 831..=840 | 911..=920 => screen(0, 2, None),
        731..=740 => screen(0, 2, Some(0)),
        741..=750 => screen(0, 2, Some(1)),
        751..=830 => screen(0, 2, Some(3 + 2 * ((frame - 751) / 10) as usize)),
        841..=890 | 931..=940 => screen(1, 2, None),
        891..=910 | 921..=930 => screen(2, 2, None),
        941..=950 => screen(1, 0, None),
        951..=960 => screen(2, 0, None),
        961..=970 | 991..=1000 | 1031..=1060 => screen(0, 0, None),
        971..=980 => screen(0, 0, Some(0)),
        981..=990 => screen(0, 0, Some(1)),
        1001..=1010 => screen(1, 1, None),
        _ => MenuScreen::Command,
    }
}

fn verify_frame(world: &mut World, frame: u32) {
    assert_eq!(
        world.resource::<MenuState>().screen,
        expected(frame),
        "actor switching at {frame}"
    );
    assert_eq!(world.resource::<MenuState>().cursor, 2);
    assert_eq!(
        world.resource::<MenuOpen>().0,
        frame <= 1020 || (1031..=1070).contains(&frame)
    );
    let data = world.resource::<GameData>();
    let equipment = world.resource::<Equipment>();
    for (actor, worn) in [
        (1, [2, 0, 64, 83, 0]),
        (2, [6, 0, 65, 83, 0]),
        (9, [41, 62, 81, 103, 0]),
    ] {
        assert_eq!(equipment.slots(data.actor(actor).unwrap()), worn);
    }
    let inventory = world.resource::<Inventory>();
    for id in ARMOR.into_iter().chain([1, 2, 3]) {
        assert_eq!(inventory.count(id), 1);
    }
    if matches!(frame, 841 | 891 | 911 | 921 | 931 | 951 | 961 | 1001) {
        let scene = world.resource::<Scene>();
        assert_eq!(
            (scene.slot_frame, scene.preview, scene.picking),
            (0, None, None)
        );
        for nav in &scene.lists {
            assert_eq!(
                (nav.index, nav.offset, nav.cursor_frame, nav.arrow_frame),
                (0, 0, 0, 0)
            );
        }
    }
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let actual = checks
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let sounds = world.resource::<SystemSounds>();
        let (sound, count) = match frame - 1 {
            730 | 970 => (Some(&sounds.decision), 1),
            900 | 905 => (Some(&sounds.buzzer), 1),
            830 | 990 | 1010 | 1020 | 1060 | 1070 => (Some(&sounds.cancel), 1),
            740 | 750 | 760 | 770 | 780 | 790 | 800 | 810 | 820 | 840 | 890 | 910 | 920 | 930
            | 940 | 950 | 960 | 980 => (Some(&sounds.cursor), 1),
            1000 => (Some(&sounds.cursor), 2),
            _ => (None, 0),
        };
        let expected = sound
            .and_then(|sound| AudioRequest::se(&sound.name, sound.volume, sound.tempo))
            .map(|request| vec![request; count])
            .unwrap_or_default();
        assert_eq!(actual, expected, "actor switching sounds at {frame}");
        checks.frames += 1;
        checks.sounds += actual.len() as u32;
    });
}

pub(super) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!((checks.frames, checks.sounds), (352, 30));
    assert!(!world.resource::<MenuOpen>().0);
    info!("equipment actor switching: 352 states and 30 exact sounds verified");
}
