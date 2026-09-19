use crate::audio::{AudioRequest, SystemSounds};
use crate::menu::{MenuOpen, MenuScreen, MenuState};
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
        1210 | 1306 | 1345 | 1385 => Some(KeyCode::ArrowUp),
        1215 | 1305 | 1346 | 1386 => Some(KeyCode::ArrowDown),
        1220 | 1307 | 1348 | 1387 => Some(KeyCode::PageDown),
        1225 | 1308 | 1347 | 1388 => Some(KeyCode::PageUp),
        1309 | 1349 => Some(KeyCode::ArrowLeft),
        1310 | 1350 => Some(KeyCode::ArrowRight),
        1395 | 1400 | 1405 | 1410 => Some(KeyCode::Escape),
        _ => None,
    }
}

pub(crate) fn held_input(world: &mut World, frame: u32) -> bool {
    if !(1230..=1270).contains(&frame) {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    if frame == 1230 {
        keys.reset_all();
    }
    keys.press(KeyCode::ArrowDown);
    true
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1205 {
        assert!(!world.resource::<MenuOpen>().0);
        world.insert_resource(Checks::default());
        world.resource_mut::<MenuOpen>().0 = true;
        world.resource_mut::<MenuState>().screen = MenuScreen::ItemTarget {
            item_id: 105,
            cursor: 0,
        };
        world.resource_scope(|world, data: Mut<crate::gamedata::GameData>| {
            world
                .resource_mut::<crate::equipment::Equipment>()
                .set_slot(data.actor(1).unwrap(), 1, 0);
        });
        let mut vitals = world.resource_mut::<crate::vitals::Vitals>();
        vitals.set(1, 7, 5);
        vitals.set(2, 0, 0);
    }
    if (1206..=1412).contains(&frame) {
        verify_frame(world, frame);
    }
    match frame {
        1300 => {
            world.resource_mut::<MenuState>().screen = MenuScreen::SkillTarget {
                member: 0,
                skill_id: 9,
                cursor: 3,
            }
        }
        1340 => {
            world.resource_mut::<MenuState>().screen = MenuScreen::SkillTarget {
                member: 2,
                skill_id: 49,
                cursor: 1,
            }
        }
        1380 => {
            world.resource_mut::<MenuState>().screen = MenuScreen::SkillTarget {
                member: 1,
                skill_id: 7,
                cursor: 0,
            }
        }
        _ => {}
    }
    match frame {
        1212 => Some("target-nav-wrap"),
        1227 => Some("target-nav-page"),
        1254 => Some("target-nav-held"),
        1262 => Some("target-nav-held-wrap"),
        1315 => Some("target-nav-party"),
        1355 => Some("target-nav-self"),
        _ => None,
    }
}

fn verify_frame(world: &mut World, frame: u32) {
    let cursor = match frame {
        1206..=1210 | 1216..=1220 | 1226..=1230 | 1262..=1265 => 0,
        1211..=1215 | 1221..=1225 | 1258..=1261 => 3,
        1231..=1253 | 1266..=1269 => 1,
        _ => 2,
    };
    let expected = match frame {
        1206..=1300 => Some(MenuScreen::ItemTarget {
            item_id: 105,
            cursor,
        }),
        1301..=1340 => Some(MenuScreen::SkillTarget {
            member: 0,
            skill_id: 9,
            cursor: 3,
        }),
        1341..=1380 => Some(MenuScreen::SkillTarget {
            member: 2,
            skill_id: 49,
            cursor: 1,
        }),
        1381..=1395 => Some(MenuScreen::SkillTarget {
            member: 1,
            skill_id: 7,
            cursor: if matches!(frame, 1386 | 1388) { 3 } else { 0 },
        }),
        _ => None,
    };
    if let Some(expected) = expected {
        assert_eq!(
            world.resource::<MenuState>().screen,
            expected,
            "target navigation frame {frame}"
        );
    }
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let heard = checks
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .cloned()
            .collect::<Vec<_>>();
        let sounds = world.resource::<SystemSounds>();
        let expected = match frame {
            1211 | 1216 | 1221 | 1226 | 1231 | 1254 | 1258 | 1262 | 1266 | 1270 | 1386..=1389 => {
                Some(&sounds.cursor)
            }
            1396 | 1401 | 1406 | 1411 => Some(&sounds.cancel),
            _ => None,
        };
        let expected = expected
            .and_then(|sound| AudioRequest::se(&sound.name, sound.volume, sound.tempo))
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(heard, expected, "target navigation sounds at {frame}");
        checks.frames += 1;
        checks.sounds += expected.len() as u32;
    });
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!((checks.frames, checks.sounds), (207, 18));
    assert!(!world.resource::<MenuOpen>().0);
    info!(
        "target navigation: 207 input frames, 18 exact sounds, held wrapping and fixed scopes verified"
    );
}
