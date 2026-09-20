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

pub(crate) fn held_input(world: &mut World, frame: u32) -> bool {
    if !(1010..=1090).contains(&frame) {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    if frame == 1010 {
        keys.reset_all();
    }
    keys.press(KeyCode::ArrowDown);
    true
}

pub(super) fn drive(world: &mut World, frame: u32) {
    if frame == 1010 {
        assert!(!world.resource::<MenuOpen>().0);
        world.insert_resource(Checks::default());
        let mut skills = (1..=24).collect::<Vec<_>>();
        skills.push(49);
        world
            .resource_mut::<crate::progression::Progression>()
            .load_skills(vec![(1, skills)]);
        world.insert_resource(crate::state::Inventory::default());
        for id in 1..=10 {
            world
                .resource_mut::<crate::state::Inventory>()
                .add_item(id, 1);
        }
    }
    if (1011..=1093).contains(&frame) {
        verify_frame(world, frame);
    }
    match frame {
        1020 | 1060 => {
            world.resource_mut::<MenuOpen>().0 = true;
            world.resource_mut::<MenuState>().screen = MenuScreen::SkillList {
                member: 0,
                cursor: if frame == 1020 { 0 } else { 4 },
            };
        }
        1035 => {
            let now = world.resource::<crate::timing::GameFrames>().frame;
            assert!(
                world
                    .resource_mut::<crate::transitions::Transition>()
                    .start_for(
                        crate::transitions::Kind::Fade,
                        true,
                        now,
                        IVec2::new(160, 120),
                        60
                    )
            );
        }
        1042 => world
            .resource_mut::<crate::transitions::Transition>()
            .clear(),
        1047 => {
            world.resource_mut::<MenuState>().screen = MenuScreen::SkillTarget {
                member: 0,
                skill_id: 49,
                cursor: 0,
            };
        }
        1070 | 1093 => {
            world.resource_mut::<MenuOpen>().0 = false;
            world.resource_mut::<MenuState>().screen = MenuScreen::Command;
        }
        1080 => {
            world.resource_mut::<MenuOpen>().0 = true;
            world.resource_mut::<MenuState>().screen = MenuScreen::ItemList { cursor: 0 };
        }
        _ => {}
    }
}

fn verify_frame(world: &mut World, frame: u32) {
    let expected = match frame {
        1021..=1047 => MenuScreen::SkillList {
            member: 0,
            cursor: if frame < 1034 {
                0
            } else if frame < 1046 {
                2
            } else {
                4
            },
        },
        1048..=1060 => MenuScreen::SkillTarget {
            member: 0,
            skill_id: 49,
            cursor: 0,
        },
        1061..=1070 => MenuScreen::SkillList {
            member: 0,
            cursor: if frame < 1062 {
                4
            } else if frame < 1066 {
                6
            } else if frame < 1070 {
                8
            } else {
                10
            },
        },
        1081..=1093 => MenuScreen::ItemList {
            cursor: if frame < 1082 {
                0
            } else if frame < 1086 {
                2
            } else if frame < 1090 {
                4
            } else {
                6
            },
        },
        _ => MenuScreen::Command,
    };
    assert_eq!(
        world.resource::<MenuState>().screen,
        expected,
        "list hold frame {frame}"
    );
    assert_eq!(
        world.resource::<MenuOpen>().0,
        expected != MenuScreen::Command
    );
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let actual = checks
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let cursor = &world.resource::<SystemSounds>().cursor;
        let expected = matches!(frame, 1034 | 1046 | 1062 | 1066 | 1070 | 1082 | 1086 | 1090)
            .then(|| AudioRequest::se(&cursor.name, cursor.volume, cursor.tempo).unwrap())
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "list hold sounds at {frame}");
        checks.frames += 1;
        checks.sounds += actual.len() as u32;
    });
}

pub(super) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!((checks.frames, checks.sounds), (83, 8));
    info!(
        "list repeat: 83 states and eight exact sounds verified across closed menus, transitions and fixed targets"
    );
}
