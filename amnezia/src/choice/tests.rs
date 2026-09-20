use super::*;
use crate::audio::{AudioRequest, SystemSounds};
use crate::timing::GameFrames;

fn app(count: usize, cancel: i32) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Choice>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<GameFrames>()
        .init_resource::<crate::menu::DirectionInput>()
        .init_resource::<crate::menu::MenuOpen>()
        .add_message::<AudioRequest>()
        .insert_resource(SystemSounds {
            cursor: sound("CURSOR"),
            decision: sound("DECISION"),
            cancel: sound("CANCEL"),
            ..default()
        })
        .add_systems(
            Update,
            (crate::menu::update_directions, choice_input).chain(),
        );
    app.update();
    app.world_mut().resource_mut::<Choice>().open(
        (0..count).map(|i| i.to_string()).collect(),
        3,
        cancel,
    );
    app
}

fn sound(name: &str) -> amnezia_data::SoundDef {
    amnezia_data::SoundDef {
        name: name.into(),
        volume: 100,
        tempo: 100,
        ..default()
    }
}

fn heard(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

fn expected(names: &[&str]) -> Vec<AudioRequest> {
    names
        .iter()
        .map(|name| AudioRequest::se(name, 100, 100).unwrap())
        .collect()
}

fn press(app: &mut App, keys: &[KeyCode]) -> Vec<AudioRequest> {
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.reset_all();
    for key in keys {
        input.press(*key);
    }
    app.update();
    heard(app)
}

#[test]
fn arrows_wrap_and_repeat_at_the_same_24_and_four_tick_boundaries_at_all_render_rates() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = app(4, 0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        let mut count = 0;
        for frame in 1..=fps {
            let tick = frame * 60 / fps;
            app.world_mut().resource_mut::<GameFrames>().frame = tick;
            app.update();
            let moves = 1 + if tick < 24 { 0 } else { 1 + (tick - 24) / 4 };
            assert_eq!(
                app.world().resource::<Choice>().cursor,
                moves as usize % 4,
                "{fps} FPS, tick {tick}"
            );
            let sounds = heard(&mut app);
            assert!(
                sounds
                    .iter()
                    .all(|sound| *sound == expected(&["CURSOR"])[0])
            );
            count += sounds.len();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
        }
        assert_eq!(count, 11);
    }
}

#[test]
fn page_keys_jump_to_the_four_row_page_edges_and_boundaries_and_lateral_keys_are_silent() {
    let mut app = app(4, 0);
    assert_eq!(press(&mut app, &[KeyCode::PageDown]), expected(&["CURSOR"]));
    assert_eq!(app.world().resource::<Choice>().cursor, 3);
    assert!(
        press(
            &mut app,
            &[KeyCode::PageDown, KeyCode::ArrowLeft, KeyCode::ArrowRight]
        )
        .is_empty()
    );
    assert_eq!(press(&mut app, &[KeyCode::PageUp]), expected(&["CURSOR"]));
    assert_eq!(app.world().resource::<Choice>().cursor, 0);
    assert!(press(&mut app, &[KeyCode::PageUp]).is_empty());
}

#[test]
fn window_navigation_precedes_decision_and_each_direction_keeps_its_sound() {
    let mut app = app(4, 0);
    let sounds = press(
        &mut app,
        &[
            KeyCode::ArrowDown,
            KeyCode::ArrowUp,
            KeyCode::PageDown,
            KeyCode::PageUp,
            KeyCode::Enter,
            KeyCode::Space,
        ],
    );
    assert_eq!(
        sounds,
        expected(&["CURSOR", "CURSOR", "CURSOR", "CURSOR", "DECISION"])
    );
    let choice = app.world().resource::<Choice>();
    assert_eq!(choice.result, Some(0));
    assert_eq!(choice.indent, 3);
    assert!(!choice.active());
}

#[test]
fn cancel_keeps_priority_over_decision_including_when_it_is_disallowed() {
    for cancel in 0..=5 {
        let mut app = app(4, cancel);
        let sounds = press(
            &mut app,
            &[KeyCode::PageDown, KeyCode::Escape, KeyCode::Enter],
        );
        let choice = app.world().resource::<Choice>();
        assert_eq!(choice.cursor, 3);
        assert_eq!(choice.result, (cancel > 0).then_some(cancel - 1));
        assert_eq!(choice.active(), cancel == 0);
        assert_eq!(
            sounds,
            expected(if cancel == 0 {
                &["CURSOR"]
            } else {
                &["CURSOR", "CANCEL"]
            })
        );
    }
}

#[test]
fn single_choice_arrows_still_sound_but_an_empty_choice_does_not_accept_input() {
    for count in [0, 1] {
        let mut app = app(count, 0);
        let sounds = press(
            &mut app,
            &[
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
                KeyCode::PageDown,
                KeyCode::PageUp,
            ],
        );
        assert_eq!(
            sounds,
            expected(if count == 0 {
                &[]
            } else {
                &["CURSOR", "CURSOR"]
            })
        );
        assert_eq!(app.world().resource::<Choice>().cursor, 0);
        if count == 0 {
            assert!(press(&mut app, &[KeyCode::Enter]).is_empty());
            assert!(app.world().resource::<Choice>().active());
            assert_eq!(app.world().resource::<Choice>().result, None);
        }
    }
}

#[test]
fn the_message_window_never_navigates_more_than_its_four_original_choices() {
    let mut app = app(6, 0);
    app.world_mut().resource_mut::<Choice>().cursor = 3;
    assert_eq!(
        press(&mut app, &[KeyCode::ArrowDown]),
        expected(&["CURSOR"])
    );
    assert_eq!(app.world().resource::<Choice>().cursor, 0);
    assert_eq!(
        press(&mut app, &[KeyCode::ArrowUp, KeyCode::Enter]),
        expected(&["CURSOR", "DECISION"])
    );
    assert_eq!(app.world().resource::<Choice>().result, Some(3));
}

#[test]
fn closed_and_paused_choices_keep_the_global_hold_phase_without_replaying_moves() {
    let mut app = app(4, 0);
    app.world_mut().resource_mut::<Choice>().active = false;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    for frame in 1..=64 {
        if frame == 31 {
            app.world_mut().resource_mut::<Choice>().active = true;
        }
        app.world_mut().resource_mut::<crate::menu::MenuOpen>().0 = (33..=60).contains(&frame);
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
        assert_eq!(
            app.world().resource::<Choice>().cursor,
            if frame < 32 {
                0
            } else if frame < 64 {
                1
            } else {
                2
            },
            "tick {frame}"
        );
        assert_eq!(
            heard(&mut app),
            expected(if matches!(frame, 32 | 64) {
                &["CURSOR"]
            } else {
                &[]
            }),
            "tick {frame}"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }
}

#[test]
fn transitions_block_navigation_confirmation_and_cancel_without_replaying_them_afterwards() {
    let mut app = app(4, 2);
    let mut transition = crate::transitions::Transition::default();
    transition.start_for(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO, 6);
    app.insert_resource(transition);
    assert!(
        press(
            &mut app,
            &[
                KeyCode::ArrowDown,
                KeyCode::PageDown,
                KeyCode::Enter,
                KeyCode::Escape
            ]
        )
        .is_empty()
    );
    let choice = app.world().resource::<Choice>();
    assert!(choice.active());
    assert_eq!((choice.cursor, choice.result), (0, None));
    app.insert_resource(crate::transitions::Transition::default());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    assert!(heard(&mut app).is_empty());
    assert!(app.world().resource::<Choice>().active());
}

#[test]
fn render_only_updates_do_not_repeat_and_reopening_discards_the_previous_result() {
    let mut app = app(4, 0);
    press(&mut app, &[KeyCode::ArrowUp]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    for _ in 0..40 {
        app.update();
        assert_eq!(app.world().resource::<Choice>().cursor, 3);
        assert!(heard(&mut app).is_empty());
    }
    press(&mut app, &[KeyCode::Enter]);
    assert_eq!(app.world().resource::<Choice>().result, Some(3));
    app.world_mut()
        .resource_mut::<Choice>()
        .open(vec!["Igen".into(), "Nem".into()], 7, 2);
    let choice = app.world().resource::<Choice>();
    assert_eq!(
        (
            choice.cursor,
            choice.indent,
            choice.cancel_type,
            choice.result
        ),
        (0, 7, 2, None)
    );
    assert!(choice.active());
}
