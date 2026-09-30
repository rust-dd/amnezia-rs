use crate::audio::{AudioRequest, SystemSounds};
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::menu::{MenuOpen, MenuScreen, MenuState};
use crate::progression::Progression;
use crate::state::Party;
use crate::text::HeroName;
use crate::vitals::Vitals;
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) use crate::menu::view::skill_list::pixels::snapshot;
pub(crate) use repeat::held_input;

mod repeat;

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
        305 | 380 | 442 | 482 | 498 | 710 | 980 | 1000 => Some(KeyCode::Escape),
        320 | 400 | 550 | 560 | 570 | 580 | 590 | 600 | 610 | 620 | 630 | 640 | 650 => {
            Some(KeyCode::ArrowDown)
        }
        750 => Some(KeyCode::ArrowUp),
        330 | 335 | 350 | 398 | 402 | 420 | 436 | 460 | 476 | 514 | 516 | 536 | 690 | 800 => {
            Some(KeyCode::Enter)
        }
        540 | 660 => Some(KeyCode::ArrowRight),
        _ => None,
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    repeat::drive(world, frame);
    if frame == 300 {
        crate::session::clear_transient(world);
        world.insert_resource(Checks::default());
        world.insert_resource(MenuState::default());
        world.insert_resource(Progression::default());
        world.insert_resource(Equipment::default());
        world.insert_resource(HeroName("Áron".into()));
        world.resource_mut::<Party>().restore(vec![1, 2, 3, 4]);
        world.resource_mut::<Vitals>().set(1, 7, 5);
        world.resource_mut::<Vitals>().set(2, 30, 75);
    }
    if (301..=1002).contains(&frame) {
        verify_sounds(world, frame);
        verify_returns(world, frame);
    }
    if matches!(frame, 457 | 497) {
        assert_eq!(
            world.resource::<Vitals>().get_stored(1),
            Some((if frame == 457 { 57 } else { 63 }, 5))
        );
        assert_eq!(
            world.resource::<Vitals>().get_stored(2),
            Some((30, if frame == 457 { 60 } else { 45 }))
        );
    }
    if frame == 510 {
        let mut ids = (1..=24).collect::<Vec<_>>();
        ids.push(49);
        world
            .resource_mut::<Progression>()
            .load_skills(vec![(1, ids)]);
        world.resource_mut::<Vitals>().set(1, 7, 5);
    }
    if frame == 680 {
        world.resource_scope(|world, data: Mut<GameData>| {
            world
                .resource_mut::<Equipment>()
                .set_slot(data.actor(1).unwrap(), 1, 152);
        });
        world.resource_mut::<Vitals>().set(1, 7, 8);
        world.resource_mut::<Vitals>().set_states(1, vec![2]);
    }
    if frame == 740 {
        world.resource_mut::<MenuState>().screen = MenuScreen::SkillList {
            member: 0,
            cursor: 6,
        };
    }
    if frame == 780 {
        world.resource_mut::<Vitals>().set(1, 7, 0);
    }
    if frame == 900 {
        world.resource_mut::<Vitals>().set(1, 7, 8);
        world.resource_mut::<HeroName>().0 = LONG_NAME.into();
        let mut data = world.resource_mut::<GameData>();
        let skill = data.skills.iter_mut().find(|skill| skill.id == 49).unwrap();
        skill.name = LONG_NAME.into();
        skill.description = format!("{LONG_NAME} {LONG_NAME}");
        world.resource_mut::<MenuState>().screen = MenuScreen::SkillList {
            member: 0,
            cursor: 24,
        };
    }
    if frame == 950 {
        world
            .resource_mut::<Progression>()
            .load_skills(vec![(1, Vec::new())]);
    }
    match frame {
        418 => Some("skills-tiffany"),
        457 => Some("skills-used"),
        497 => Some("skills-used-again"),
        534 => Some("skills-grid"),
        642 => Some("skills-scroll-first"),
        643 => Some("skills-scroll-half"),
        645 => Some("skills-scroll-done"),
        665 => Some("skills-bottom"),
        685 => Some("skills-half-cost"),
        726 => Some("skills-returned"),
        752 => Some("skills-up-first"),
        753 => Some("skills-up-half"),
        755 => Some("skills-up-done"),
        795 => Some("skills-no-sp"),
        920 => Some("skills-long-name"),
        970 => Some("skills-cleared"),
        _ => None,
    }
}

fn verify_returns(world: &World, frame: u32) {
    if world.resource::<crate::menu::SceneFlow>().active() {
        return;
    }
    let expected = match frame {
        381..=398 | 499..=514 | 981..=1002 => MenuScreen::Command,
        399..=400 | 515..=516 => MenuScreen::MemberSelect {
            action: crate::menu::MemberAction::Skill,
            cursor: 0,
        },
        401..=402 => MenuScreen::MemberSelect {
            action: crate::menu::MemberAction::Skill,
            cursor: 1,
        },
        _ => return,
    };
    let state = world.resource::<MenuState>();
    assert_eq!(state.screen, expected, "skill return at {frame}");
    assert_eq!(state.cursor, 1);
    assert_eq!(world.resource::<MenuOpen>().0, frame <= 1000);
}

fn verify_sounds(world: &mut World, frame: u32) {
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let actual = checks
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let sounds = world.resource::<SystemSounds>();
        let expected = match frame - 1 {
            350 | 536 | 800 => Some(&sounds.buzzer),
            // Opening from the map consumes Escape on the following stopped player update.
            306 | 330 | 335 | 398 | 402 | 420 | 460 | 514 | 516 | 690 => Some(&sounds.decision),
            380 | 442 | 482 | 498 | 710 | 980 | 1000 => Some(&sounds.cancel),
            320 | 400 | 540 | 550 | 560 | 570 | 580 | 590 | 600 | 610 | 620 | 630 | 640 | 650
            | 660 | 750 => Some(&sounds.cursor),
            _ => None,
        };
        let expected = if matches!(frame - 1, 436 | 476) {
            AudioRequest::se("Song2", 100, 110)
        } else {
            expected.and_then(|sound| AudioRequest::se(&sound.name, sound.volume, sound.tempo))
        };
        assert_eq!(
            actual,
            expected.into_iter().collect::<Vec<_>>(),
            "skill sounds at {frame}"
        );
        checks.sounds += actual.len() as u32;
        checks.frames += 1;
    });
}

pub(crate) fn verify_finished(world: &World) {
    repeat::verify_finished(world);
    let checks = world.resource::<Checks>();
    assert_eq!(checks.pixels.load(Ordering::Relaxed), 17);
    assert_eq!(checks.frames, 702);
    assert_eq!(checks.sounds, 38);
    assert!(!world.resource::<MenuOpen>().0);
    assert_eq!(world.resource::<MenuState>().screen, MenuScreen::Command);
    assert_eq!(world.resource::<Vitals>().get_stored(2), Some((30, 45)));
    info!(
        "skill menu: all window pixels, two-way scrolling, empty/disabled selection, field casting, sounds and target return verified"
    );
}
