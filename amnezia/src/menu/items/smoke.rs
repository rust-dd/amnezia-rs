use super::List;
use crate::audio::{AudioRequest, SystemSounds};
use crate::menu::{MenuOpen, MenuScreen, MenuState};
use crate::state::{Inventory, Party};
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) use crate::menu::view::item_list::pixels::snapshot;

#[derive(Resource, Default)]
pub(crate) struct Checks {
    pub pixels: Arc<AtomicUsize>,
    pub target_pixels: Arc<AtomicUsize>,
    returned: bool,
    consumed: bool,
    audio: MessageCursor<AudioRequest>,
    sounds: u8,
}

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        310 | 570 | 640 | 950 | 970 | 1160 | 1170 | 1180 | 1190 => Some(KeyCode::Escape),
        320 | 380 | 530 | 550 | 600 | 620 | 625 => Some(KeyCode::Enter),
        370 => Some(KeyCode::ArrowRight),
        390 | 670 => Some(KeyCode::ArrowLeft),
        400 | 410 | 420 | 430 | 440 | 450 | 460 | 470 | 480 | 490 | 500 | 510 => {
            Some(KeyCode::ArrowDown)
        }
        680 | 690 | 700 | 710 | 720 | 730 | 740 | 750 | 760 | 770 | 800 => Some(KeyCode::ArrowUp),
        _ => None,
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 300 {
        crate::session::clear_transient(world);
        world.insert_resource(Checks::default());
        world.insert_resource(Inventory::default());
        world.insert_resource(MenuState::default());
        world.resource_mut::<Party>().restore(vec![1]);
        world.resource_mut::<crate::vitals::Vitals>().set(1, 7, 5);
    }
    if frame == 350 {
        let mut inventory = world.resource_mut::<Inventory>();
        for id in 1..=24 {
            inventory.add_item(id, id * 3);
        }
        inventory.add_item(105, 2);
        inventory.add_gold(1234);
    }
    if frame >= 300 {
        world.resource_scope(|world, mut checks: Mut<Checks>| {
            let requests = checks
                .audio
                .read(world.resource::<Messages<AudioRequest>>())
                .cloned()
                .collect::<Vec<_>>();
            let sounds = world.resource::<SystemSounds>();
            let expected = match frame {
                381 => Some((1, &sounds.buzzer)),
                551 => Some((2, &sounds.item)),
                621 => Some((4, &sounds.item)),
                626 => Some((8, &sounds.buzzer)),
                _ => None,
            };
            if let Some((flag, sound)) = expected {
                assert_eq!(
                    requests,
                    [AudioRequest::se(&sound.name, sound.volume, sound.tempo).unwrap()],
                    "item sound at {frame}"
                );
                checks.sounds |= flag;
            }
        });
    }
    if frame == 580 {
        assert_eq!(
            world.resource::<MenuState>().screen,
            MenuScreen::ItemList { cursor: 24 }
        );
        assert_eq!(world.resource::<Inventory>().count(105), 1);
        assert_eq!(
            world.resource::<crate::vitals::Vitals>().get_stored(1),
            Some((57, 5))
        );
        world.resource_mut::<Checks>().returned = true;
    }
    if frame == 650 {
        assert_eq!(
            world.resource::<MenuState>().screen,
            MenuScreen::ItemList { cursor: 23 }
        );
        assert_eq!(world.resource::<Inventory>().count(105), 0);
        assert_eq!(
            world.resource::<crate::vitals::Vitals>().get_stored(1),
            Some((63, 5))
        );
        world.resource_mut::<Checks>().consumed = true;
    }
    if frame == 630 {
        assert_eq!(
            world.resource::<MenuState>().screen,
            MenuScreen::ItemTarget {
                item_id: 105,
                cursor: 0
            }
        );
        assert_eq!(world.resource::<Inventory>().count(105), 0);
        assert_eq!(
            world.resource::<crate::vitals::Vitals>().get_stored(1),
            Some((63, 5))
        );
    }
    if frame == 900 {
        world.insert_resource(Inventory::default());
    }
    if let Some(label) = crate::menu::view::target::smoke::drive(world, frame) {
        return Some(label);
    }
    match frame {
        330 => Some("items-empty"),
        385 => Some("items-disabled"),
        512 => Some("items-scroll-first"),
        513 => Some("items-scroll-half"),
        515 => Some("items-scroll-done"),
        580 => Some("items-returned"),
        650 => Some("items-depleted"),
        803 => Some("items-up-half"),
        805 => Some("items-up-done"),
        910 => Some("items-cleared"),
        _ => None,
    }
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!(checks.pixels.load(Ordering::Relaxed), 11);
    assert_eq!(checks.target_pixels.load(Ordering::Relaxed), 8);
    assert!(checks.returned && checks.consumed);
    assert_eq!(checks.sounds, 15);
    assert!(!world.resource::<MenuOpen>().0);
    assert_eq!(world.resource::<List>().navigation.index, 0);
    assert_eq!(world.resource::<Inventory>().count(105), 0);
    info!(
        "item menu: original grid, descriptions, scrolling, disabled choices, target return and item consumption verified"
    );
}
