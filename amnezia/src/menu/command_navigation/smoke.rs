use crate::audio::{AudioRequest, SystemSounds};
use crate::menu::{MemberAction, MenuOpen, MenuScreen, MenuState};
use crate::state::Party;
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
struct Checks {
    audio: MessageCursor<AudioRequest>,
    frames: u32,
    sounds: u32,
    pixels: Arc<AtomicUsize>,
}

pub(crate) fn pictures(world: &World) -> Arc<AtomicUsize> {
    world.resource::<Checks>().pixels.clone()
}

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        1220 | 1290 | 1320 | 1400 => Some(KeyCode::PageUp),
        1230 | 1440 => Some(KeyCode::ArrowUp),
        1240 | 1270 | 1280 | 1330 | 1460 => Some(KeyCode::ArrowDown),
        1250 | 1300 | 1390 | 1450 => Some(KeyCode::PageDown),
        1251 | 1304 | 1340 => Some(KeyCode::Enter),
        1410 | 1412 | 1470 | 1480 => Some(KeyCode::Escape),
        _ => None,
    }
}

pub(crate) fn held_input(world: &mut World, frame: u32) -> bool {
    let (start, key) = if (1180..1212).contains(&frame) {
        (1180, KeyCode::ArrowDown)
    } else if (1350..1382).contains(&frame) {
        (1350, KeyCode::ArrowUp)
    } else {
        return false;
    };
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    if frame == start {
        keys.reset_all();
    }
    keys.press(key);
    true
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1170 {
        assert!(!world.resource::<MenuOpen>().0);
        world.insert_resource(Checks::default());
        world.insert_resource(crate::vitals::Vitals::default());
        world.resource_mut::<Party>().restore(vec![1, 2, 3, 4]);
        world.resource_mut::<MenuOpen>().0 = true;
        world.insert_resource(MenuState::default());
    }
    if (1171..=1498).contains(&frame) {
        verify_frame(world, frame);
    }
    if frame == 1430 {
        assert!(!world.resource::<MenuOpen>().0);
        world.resource_mut::<MenuOpen>().0 = true;
        world.resource_mut::<Party>().restore(vec![1]);
        world.insert_resource(MenuState {
            cursor: 2,
            screen: member(MemberAction::Equip, 0),
        });
    }
    match frame {
        1215 => Some("menu-navigation-command-last"),
        1275 => Some("menu-navigation-end-yes"),
        1395 => Some("menu-navigation-member-last"),
        1465 => Some("menu-navigation-single-member"),
        _ => None,
    }
}

fn member(action: MemberAction, cursor: usize) -> MenuScreen {
    MenuScreen::MemberSelect { action, cursor }
}

fn expected(frame: u32) -> (usize, MenuScreen) {
    let screen = match frame {
        1252..=1270 | 1281..=1290 | 1301..=1304 => MenuScreen::EndGame { cursor: 1 },
        1271..=1280 | 1291..=1300 => MenuScreen::EndGame { cursor: 0 },
        1341..=1350 | 1382..=1390 | 1401..=1410 => member(MemberAction::Skill, 0),
        1351..=1373 | 1391..=1400 => member(MemberAction::Skill, 3),
        1374..=1377 => member(MemberAction::Skill, 2),
        1378..=1381 => member(MemberAction::Skill, 1),
        1431..=1470 => member(MemberAction::Equip, 0),
        _ => MenuScreen::Command,
    };
    let cursor = match frame {
        1181..=1203 | 1331..=1430 => 1,
        1204..=1207 | 1431..=1498 => 2,
        1208..=1211 => 3,
        1212..=1220 | 1231..=1240 | 1251..=1320 => 4,
        _ => 0,
    };
    (cursor, screen)
}

fn verify_frame(world: &mut World, frame: u32) {
    let state = world.resource::<MenuState>();
    if !world.resource::<crate::menu::SceneFlow>().active() {
        assert_eq!(
            (state.cursor, state.screen),
            expected(frame),
            "menu navigation at {frame}"
        );
        assert_eq!(
            world.resource::<MenuOpen>().0,
            frame <= 1412 || (1431..=1480).contains(&frame)
        );
    }
    assert!(!world.resource::<crate::title::TitleActive>().0);
    assert!(
        !world
            .resource::<crate::menu::save_files::SaveFiles>()
            .active()
    );
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let actual = checks
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let sounds = world.resource::<SystemSounds>();
        let sound = match frame - 1 {
            1180 | 1203 | 1207 | 1211 | 1220 | 1230 | 1240 | 1250 | 1270 | 1280 | 1290 | 1300
            | 1320 | 1330 | 1350 | 1373 | 1377 | 1381 | 1390 | 1400 | 1440 | 1460 => {
                Some(&sounds.cursor)
            }
            1251 | 1304 | 1340 => Some(&sounds.decision),
            1410 | 1412 | 1470 | 1480 => Some(&sounds.cancel),
            _ => None,
        };
        let expected = sound
            .and_then(|sound| AudioRequest::se(&sound.name, sound.volume, sound.tempo))
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "menu navigation sounds at {frame}");
        checks.frames += 1;
        checks.sounds += actual.len() as u32;
    });
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!((checks.frames, checks.sounds), (328, 29));
    assert_eq!(checks.pixels.load(Ordering::Relaxed), 4);
    assert!(!world.resource::<MenuOpen>().0);
    info!("main menu navigation: 328 states, 29 exact sounds and four cursor images verified");
}
