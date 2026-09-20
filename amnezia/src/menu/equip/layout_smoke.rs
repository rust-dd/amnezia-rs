use crate::audio::{AudioRequest, SystemSounds};
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::menu::{MemberAction, MenuOpen, MenuScreen, MenuState};
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::text::HeroName;
use crate::vitals::Vitals;
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) use crate::menu::view::equipment::pixels::snapshot;

pub(crate) const ARMOR: [u32; 18] = [
    64, 65, 68, 69, 70, 71, 74, 75, 76, 77, 78, 79, 80, 153, 154, 155, 156, 157,
];
pub(crate) const LONG_NAME: &str = "Árvíztűrő tükörfúrógép rendkívül hosszú név";

#[derive(Resource, Default)]
pub(crate) struct Checks {
    pub pixels: Arc<AtomicUsize>,
    audio: MessageCursor<AudioRequest>,
    frames: u32,
    sounds: u32,
}

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        320 | 370 | 380 | 420 | 560 | 640 | 690 => Some(KeyCode::Enter),
        650 => Some(KeyCode::Space),
        330 | 430 | 520 => Some(KeyCode::ArrowRight),
        350 => Some(KeyCode::ArrowLeft),
        340 | 400 | 410 | 440 | 450 | 460 | 470 | 480 | 490 | 500 | 510 | 550 | 610 => {
            Some(KeyCode::ArrowDown)
        }
        540 | 600 => Some(KeyCode::ArrowUp),
        390 | 580 => Some(KeyCode::PageDown),
        590 => Some(KeyCode::PageUp),
        395 | 530 | 570 | 680 | 700 | 710 => Some(KeyCode::Escape),
        _ => None,
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 300 {
        crate::session::clear_transient(world);
        world.insert_resource(Checks::default());
        world.insert_resource(Progression::default());
        world.insert_resource(Equipment::default());
        world.insert_resource(Vitals::default());
        world.insert_resource(Inventory::default());
        world.insert_resource(HeroName("Áron".into()));
        world.resource_mut::<Party>().restore(vec![1, 2, 3, 4]);
        let mut inventory = world.resource_mut::<Inventory>();
        inventory.add_item(2, 2);
        inventory.add_item(3, 1);
        for id in ARMOR {
            inventory.add_item(id, 1);
        }
        world.insert_resource(MenuOpen(true));
        world.insert_resource(MenuState {
            cursor: 2,
            screen: screen(0, None),
        });
    }
    if (301..=712).contains(&frame) {
        verify_state(world, frame);
    }
    if frame == 620 {
        world.resource_mut::<Party>().restore(vec![9]);
    }
    if frame == 660 {
        world.resource_mut::<Party>().restore(vec![1, 2, 3, 4]);
        world.resource_mut::<HeroName>().0 = LONG_NAME.into();
        let mut data = world.resource_mut::<GameData>();
        let item = data.items.iter_mut().find(|item| item.id == 2).unwrap();
        item.name = LONG_NAME.into();
        item.description = format!("{LONG_NAME} {LONG_NAME}");
    }
    match frame {
        315 => Some("equipment-slots"),
        325 => Some("equipment-preview"),
        331 => Some("equipment-preview-lag"),
        332 => Some("equipment-preview-changed"),
        372 => Some("equipment-equipped"),
        385 => Some("equipment-weaker"),
        415 => Some("equipment-armor"),
        492 => Some("equipment-scroll-first"),
        493 => Some("equipment-scroll-half"),
        495 => Some("equipment-scroll-done"),
        525 => Some("equipment-unequip"),
        535 => Some("equipment-inactive"),
        565 => Some("equipment-reopened"),
        585 => Some("equipment-accessory"),
        605 => Some("equipment-wrap"),
        630 => Some("equipment-fixed"),
        670 => Some("equipment-long"),
        685 => Some("equipment-main-return"),
        695 => Some("equipment-member-reopen"),
        _ => None,
    }
}

fn screen(slot: usize, picking: Option<usize>) -> MenuScreen {
    MenuScreen::Equip {
        member: 0,
        slot,
        picking,
    }
}

fn expected(frame: u32) -> MenuScreen {
    match frame {
        301..=320 | 371..=380 | 396..=400 | 591..=600 | 611..=680 => screen(0, None),
        321..=330 | 351..=370 | 381..=395 => screen(0, Some(0)),
        331..=350 => screen(0, Some(1)),
        401..=410 | 541..=550 => screen(1, None),
        411..=420 | 531..=540 | 551..=560 | 571..=580 => screen(2, None),
        421..=430 | 561..=570 => screen(2, Some(0)),
        431..=520 => screen(2, Some(1 + 2 * ((frame - 431) / 10) as usize)),
        521..=530 => screen(2, Some(18)),
        581..=590 | 601..=610 => screen(4, None),
        691..=700 => MenuScreen::MemberSelect {
            action: MemberAction::Equip,
            cursor: 0,
        },
        _ => MenuScreen::Command,
    }
}

fn verify_state(world: &mut World, frame: u32) {
    assert_eq!(
        world.resource::<MenuState>().screen,
        expected(frame),
        "equipment frame {frame}"
    );
    assert_eq!(world.resource::<MenuOpen>().0, frame <= 710);
    assert_eq!(world.resource::<MenuState>().cursor, 2);
    let data = world.resource::<GameData>();
    let worn = world.resource::<Equipment>();
    assert_eq!(
        worn.slots(data.actor(1).unwrap()),
        [if frame <= 370 { 1 } else { 2 }, 0, 64, 83, 0]
    );
    assert_eq!(worn.slots(data.actor(9).unwrap()), [41, 62, 81, 103, 0]);
    let inventory = world.resource::<Inventory>();
    assert_eq!(inventory.count(1), u32::from(frame > 370));
    assert_eq!(inventory.count(2), if frame <= 370 { 2 } else { 1 });
    for id in ARMOR {
        assert_eq!(inventory.count(id), 1);
    }
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let actual = checks
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let sounds = world.resource::<SystemSounds>();
        let sound = match frame - 1 {
            320 | 370 | 380 | 420 | 560 | 690 => Some(&sounds.decision),
            330 | 350 | 400 | 410 | 430 | 440 | 450 | 460 | 470 | 480 | 490 | 500 | 510 | 520
            | 540 | 550 | 580 | 590 | 600 | 610 => Some(&sounds.cursor),
            395 | 530 | 570 | 680 | 700 | 710 => Some(&sounds.cancel),
            640 | 650 => Some(&sounds.buzzer),
            _ => None,
        };
        let expected =
            sound.and_then(|sound| AudioRequest::se(&sound.name, sound.volume, sound.tempo));
        assert_eq!(
            actual,
            expected.into_iter().collect::<Vec<_>>(),
            "equipment sounds at {frame}"
        );
        checks.frames += 1;
        checks.sounds += actual.len() as u32;
    });
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!(checks.pixels.load(Ordering::Relaxed), 18);
    assert_eq!((checks.frames, checks.sounds), (412, 34));
    assert!(!world.resource::<MenuOpen>().0);
    assert_eq!(world.resource::<MenuState>().screen, MenuScreen::Command);
    info!(
        "equipment menu: 18 full-canvas references, 412 state/audio frames and 34 exact sounds verified"
    );
}
