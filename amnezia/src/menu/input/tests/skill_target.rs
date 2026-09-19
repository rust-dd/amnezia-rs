use super::*;
use crate::animation::AnimationLibrary;
use amnezia_data::{AnimationDef, AnimationTimingDef, Learning};

fn target() -> App {
    let mut app = app_on(
        1,
        MenuScreen::SkillList {
            member: 0,
            cursor: 1,
        },
    );
    let mut heal = testkit::heal_skill(2, "Gyógyítás", 8, 20);
    heal.animation_id = 8;
    let mut data = app.world_mut().resource_mut::<GameData>();
    data.skills.push(heal);
    data.actors[0].learnings = vec![
        Learning {
            level: 1,
            skill_id: 1,
        },
        Learning {
            level: 1,
            skill_id: 2,
        },
    ];
    app.world_mut().resource_mut::<Vitals>().set(1, 7, 30);
    press_frame(&mut app, KeyCode::Enter);
    let sound = |name: &str| SoundDef {
        name: name.into(),
        volume: 100,
        tempo: 100,
        ..default()
    };
    app.insert_resource(SystemSounds {
        decision: sound("DECISION"),
        cancel: sound("CANCEL"),
        buzzer: sound("BUZZER"),
        ..default()
    });
    app.insert_resource(AnimationLibrary(vec![AnimationDef {
        id: 8,
        name: "Field heal".into(),
        animation_name: String::new(),
        scope: 0,
        position: 1,
        frames: Vec::new(),
        timings: vec![
            timing("", 0),
            timing("(OFF)", 1),
            timing("FIELD", 90),
            timing("LATER", 0),
        ],
    }]));
    app.add_message::<crate::animation::PlayAnimation>();
    app
}

fn timing(name: &str, frame: u32) -> AnimationTimingDef {
    AnimationTimingDef {
        frame,
        se_name: name.into(),
        se_volume: 55,
        se_tempo: 125,
        flash_scope: 2,
        flash_red: 31,
        flash_green: 31,
        flash_blue: 31,
        flash_power: 31,
    }
}

fn heard(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

#[test]
fn successful_field_casts_keep_the_target_open_until_cancel_returns_to_the_same_skill() {
    let mut app = target();
    for (key, expected) in [(KeyCode::Enter, (27, 22)), (KeyCode::Space, (47, 14))] {
        press_frame(&mut app, key);
        assert_eq!(
            app.world().resource::<Vitals>().get_stored(1),
            Some(expected)
        );
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::SkillTarget {
                member: 0,
                skill_id: 2,
                cursor: 0
            }
        );
    }
    press_frame(&mut app, KeyCode::Escape);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::SkillList {
            member: 0,
            cursor: 1
        }
    );
}

#[test]
fn successful_field_cast_plays_only_the_first_enabled_animation_sound_immediately() {
    let mut app = target();
    press_frame(&mut app, KeyCode::Enter);
    assert_eq!(
        heard(&mut app),
        [AudioRequest::se("FIELD", 55, 125).unwrap()]
    );
    assert!(
        app.world()
            .resource::<Messages<crate::animation::PlayAnimation>>()
            .is_empty()
    );
}

#[test]
fn skill_use_precedes_cancel_when_confirm_and_escape_arrive_together() {
    let mut app = target();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    keys.press(KeyCode::Enter);
    keys.press(KeyCode::Escape);
    app.update();
    assert_eq!(
        app.world().resource::<Vitals>().get_stored(1),
        Some((27, 22))
    );
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::SkillList {
            member: 0,
            cursor: 1
        }
    );
    assert_eq!(
        heard(&mut app),
        [
            AudioRequest::se("FIELD", 55, 125).unwrap(),
            AudioRequest::se("CANCEL", 100, 100).unwrap()
        ]
    );
}

#[test]
fn failed_skill_uses_only_buzz_and_do_not_change_the_target_or_pools() {
    for pools in [(63, 30), (7, 7), (0, 30)] {
        let mut app = target();
        app.world_mut()
            .resource_mut::<Vitals>()
            .set(1, pools.0, pools.1);
        press_frame(&mut app, KeyCode::Enter);
        assert_eq!(
            heard(&mut app),
            [AudioRequest::se("BUZZER", 100, 100).unwrap()]
        );
        assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some(pools));
        assert!(matches!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::SkillTarget { .. }
        ));
    }
}

#[test]
fn silent_missing_or_zero_volume_animation_does_not_fall_back_to_decision() {
    for kind in 0..4 {
        let mut app = target();
        let mut library = app.world_mut().resource_mut::<AnimationLibrary>();
        match kind {
            0 => library.0[0].timings = vec![timing("", 0), timing("(OFF)", 1)],
            1 => library.0.clear(),
            2 => library.0[0].timings[2].se_volume = 0,
            _ => library.0[0].timings.clear(),
        }
        press_frame(&mut app, KeyCode::Enter);
        assert_eq!(
            app.world().resource::<Vitals>().get_stored(1),
            Some((27, 22))
        );
        assert!(heard(&mut app).is_empty(), "kind {kind}");
    }
}

#[test]
fn target_transition_blocks_skill_use_navigation_and_cancel() {
    let mut app = target();
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
    app.insert_resource(transition);
    for key in [
        KeyCode::Enter,
        KeyCode::Space,
        KeyCode::ArrowDown,
        KeyCode::Escape,
    ] {
        press_frame(&mut app, key);
        assert!(heard(&mut app).is_empty());
        assert_eq!(
            app.world().resource::<Vitals>().get_stored(1),
            Some((7, 30))
        );
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::SkillTarget {
                member: 0,
                skill_id: 2,
                cursor: 0
            }
        );
    }
}

#[test]
fn the_original_healing_songs_play_their_first_animation_sound_and_spend_sp_once() {
    for (skill_id, name, volume, tempo, remaining_sp) in [
        (7, "Song2", 100, 110, 60),
        (8, "Cure2", 90, 100, 35),
        (9, "Cure3", 100, 150, 15),
    ] {
        let mut app = target();
        app.add_plugins(crate::gamedata::GameDataPlugin);
        app.insert_resource(AnimationLibrary(crate::assets::load_ron(&format!(
            "{}/animations.ron",
            crate::assets::asset_root()
        ))));
        app.world_mut().resource_mut::<Party>().restore(vec![1, 2]);
        app.world_mut().resource_mut::<Vitals>().set(1, 7, 30);
        app.world_mut().resource_mut::<Vitals>().set(2, 30, 75);
        app.world_mut().resource_mut::<MenuState>().screen = MenuScreen::SkillTarget {
            member: 1,
            skill_id,
            cursor: 0,
        };
        press_frame(&mut app, KeyCode::Enter);
        assert_eq!(
            heard(&mut app),
            [AudioRequest::se(name, volume, tempo).unwrap()]
        );
        assert_eq!(
            app.world().resource::<Vitals>().get_stored(1),
            Some((if skill_id == 7 { 57 } else { 63 }, 30))
        );
        assert_eq!(
            app.world().resource::<Vitals>().get_stored(2),
            Some((if skill_id == 9 { 38 } else { 30 }, remaining_sp))
        );
        assert!(matches!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::SkillTarget { .. }
        ));
        assert!(
            app.world()
                .resource::<Messages<crate::animation::PlayAnimation>>()
                .is_empty()
        );
    }
}
