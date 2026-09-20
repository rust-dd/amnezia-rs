use super::{Phase, Screen, ShopOpen};
use crate::audio::{AudioRequest, SystemSounds};
use crate::gamedata::GameData;
use crate::interpreter::RunningEvent;
use crate::state::{Inventory, Switches};
use crate::timing::{GameFrames, SceneFrames};
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;

pub(in crate::shop) mod layout;
mod source;
pub(crate) use super::view::pixels::snapshot;

#[derive(Resource, Default)]
struct Probe {
    audio: MessageCursor<AudioRequest>,
    bought_at: Option<u32>,
    sold_at: Option<u32>,
    returned: [bool; 2],
    held_checks: u32,
    sounds: u32,
    finished: bool,
}

pub(crate) fn input(world: &mut World, frame: u32) -> bool {
    if frame < 300 {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    if (360..392).contains(&frame) {
        if frame == 360 {
            keys.reset_all();
        }
        keys.press(KeyCode::ArrowRight);
        return true;
    }
    keys.reset_all();
    for key in match frame {
        310 | 330 | 510 | 520 | 540 | 580 => vec![KeyCode::Enter],
        320 | 355 | 500 => vec![KeyCode::ArrowDown],
        340 | 550 => vec![KeyCode::ArrowUp],
        345 | 530 | 555 | 560 | 565 | 570 => vec![KeyCode::ArrowRight],
        350 => vec![KeyCode::ArrowLeft],
        400 => vec![KeyCode::ArrowUp, KeyCode::Enter],
        420..=426 => vec![KeyCode::ArrowDown, KeyCode::Enter],
        490 | 650 | 660 | 710 => vec![KeyCode::Escape],
        _ => layout::input(frame),
    } {
        keys.press(key);
    }
    true
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 300 {
        assert!(!world.resource::<RunningEvent>().active());
        let mut inventory = Inventory::default();
        inventory.add_gold(999_999);
        inventory.add_item(1, 1);
        assert_eq!(world.resource::<GameData>().item(1).unwrap().price, 0);
        world.insert_resource(inventory);
        world.resource_mut::<crate::dialogue::Dialogue>().face = default();
        world.init_resource::<layout::Checks>();
        let mut probe = Probe::default();
        probe
            .audio
            .clear(world.resource::<Messages<AudioRequest>>());
        world.insert_resource(probe);
        world
            .resource_mut::<RunningEvent>()
            .start(0, source::commands(9021));
        return None;
    }
    if (301..=750).contains(&frame) {
        verify_sounds(world, frame);
        verify_confirmation(world);
    }
    if frame == 420 {
        let frame = world.resource::<GameFrames>().frame;
        assert!(
            world
                .resource_mut::<crate::transitions::Transition>()
                .start_for(
                    crate::transitions::Kind::Fade,
                    false,
                    frame,
                    IVec2::new(160, 120),
                    6,
                )
        );
    }
    match frame {
        305 => Some("shop-command"),
        342 => {
            assert_number(world, 11);
            Some("shop-buy-quantity")
        }
        394 => {
            assert_number(world, 5);
            None
        }
        405 => {
            assert_eq!(world.resource::<Inventory>().count(7), 15);
            Some("shop-bought")
        }
        480 => {
            assert!(matches!(phase(world), Phase::Buy { cursor: 1 }));
            Some("shop-buy-return")
        }
        522 => {
            assert!(matches!(phase(world), Phase::Sell { cursor: 0 }));
            assert_eq!(world.resource::<Inventory>().count(1), 1);
            Some("shop-disabled-sale")
        }
        575 => {
            assert_number(world, 15);
            Some("shop-sell-quantity")
        }
        585 => {
            assert_eq!(world.resource::<Inventory>().count(7), 0);
            Some("shop-sold")
        }
        645 => {
            assert!(matches!(phase(world), Phase::Sell { cursor: 0 }));
            None
        }
        680 => {
            assert!(!world.resource::<ShopOpen>().0);
            assert!(!world.resource::<RunningEvent>().active());
            assert!(world.resource::<Switches>().get(9021));
            let price = world.resource::<GameData>().item(7).unwrap().price as i32;
            assert_eq!(
                world.resource::<Inventory>().gold(),
                999_999 - price * 15 + (price / 2) * 15
            );
            None
        }
        700 => {
            world.resource_mut::<Switches>().set(9022, true);
            world
                .resource_mut::<RunningEvent>()
                .start(0, source::commands(9022));
            None
        }
        750 => {
            assert!(!world.resource::<Switches>().get(9022));
            assert!(!world.resource::<ShopOpen>().0);
            assert!(!world.resource::<RunningEvent>().active());
            assert_eq!(world.resource::<Inventory>().count(1), 1);
            assert_eq!(world.resource::<Inventory>().count(7), 0);
            world.resource_mut::<Probe>().finished = true;
            None
        }
        _ => layout::drive(world, frame),
    }
}

fn phase(world: &World) -> &Phase {
    let Screen::Shop(state) = world.resource::<Screen>() else {
        panic!("expected open shop");
    };
    &state.phase
}

fn assert_number(world: &World, expected: u32) {
    let Phase::Number(number) = phase(world) else {
        panic!("expected quantity window");
    };
    assert_eq!(number.count, expected);
    assert_eq!(number.item_id, 7);
}

fn verify_confirmation(world: &mut World) {
    let now = world.resource::<SceneFrames>().frame;
    let returned = match world.resource::<Screen>() {
        Screen::Shop(state) => match state.phase {
            Phase::Buy { .. } => Some(0),
            Phase::Sell { .. } => Some(1),
            _ => None,
        },
        _ => None,
    };
    if let Some(index) = returned {
        let mut probe = world.resource_mut::<Probe>();
        if let Some(start) = [probe.bought_at, probe.sold_at][index]
            && !probe.returned[index]
        {
            assert_eq!(now.wrapping_sub(start), 60);
            probe.returned[index] = true;
        }
    }
    let (sold, remaining) = match world.resource::<Screen>() {
        Screen::Shop(state) => match state.phase {
            Phase::Bought { remaining, .. } => (false, remaining),
            Phase::Sold { remaining, .. } => (true, remaining),
            _ => return,
        },
        _ => return,
    };
    let mut probe = world.resource_mut::<Probe>();
    let started = if sold {
        &mut probe.sold_at
    } else {
        &mut probe.bought_at
    };
    let start = *started.get_or_insert(now);
    assert_eq!(remaining, 60 - now.wrapping_sub(start));
    probe.held_checks += 1;
}

fn verify_sounds(world: &mut World, frame: u32) {
    let sounds = world.resource::<SystemSounds>();
    let cursor = &sounds.cursor;
    let expected = match frame {
        311 | 331 | 511 | 541 | 581 => vec![&sounds.decision],
        321 | 341 | 346 | 351 | 356 | 361 | 384 | 388 | 392 | 501 | 531 | 551 | 556 | 561 | 566
        | 571 => vec![cursor],
        401 => vec![cursor, &sounds.decision],
        491 | 651 | 661 | 711 => vec![&sounds.cancel],
        521 => vec![&sounds.buzzer],
        _ => Vec::new(),
    }
    .into_iter()
    .filter_map(|sound| AudioRequest::se(&sound.name, sound.volume, sound.tempo))
    .collect::<Vec<_>>();
    world.resource_scope(|world, mut probe: Mut<Probe>| {
        let actual = probe
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "shop audio at frame {frame}");
        probe.sounds += actual.len() as u32;
    });
}

pub(crate) fn verify_finished(world: &World) {
    layout::verify_finished(world);
    let probe = world.resource::<Probe>();
    assert!(probe.finished);
    assert_eq!(probe.sounds, 28);
    assert!(probe.held_checks >= 120);
    assert_eq!(probe.returned, [true; 2]);
    assert_eq!(world.resource::<Inventory>().count(1), 1);
    info!(
        "shop trade checks: {} confirmation states, {} exact sounds and transaction/cancel branches",
        probe.held_checks, probe.sounds
    );
}
