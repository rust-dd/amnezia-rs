use super::*;
use crate::menu::MemberAction;
use crate::timing::GameFrames;

fn screen(mode: u8, cursor: usize) -> MenuScreen {
    match mode {
        0 => MenuScreen::Command,
        1 => MenuScreen::MemberSelect {
            action: MemberAction::Equip,
            cursor,
        },
        _ => MenuScreen::EndGame { cursor },
    }
}

fn selected(app: &App) -> usize {
    let state = app.world().resource::<MenuState>();
    match state.screen {
        MenuScreen::Command => state.cursor,
        MenuScreen::MemberSelect { cursor, .. } | MenuScreen::EndGame { cursor } => cursor,
        other => panic!("unexpected screen {other:?}"),
    }
}

fn fixture(mode: u8, cursor: usize) -> App {
    let mut app = app_on(if mode == 0 { cursor } else { 2 }, screen(mode, cursor));
    app.world_mut().resource_mut::<GameData>().actors = (1..=4)
        .map(|id| {
            let mut actor = testkit::actor();
            actor.id = id;
            actor
        })
        .collect();
    app.world_mut()
        .resource_mut::<Party>()
        .restore(vec![1, 2, 3, 4]);
    let sound = |name: &str| SoundDef {
        name: name.into(),
        volume: 100,
        tempo: 100,
        ..default()
    };
    app.insert_resource(SystemSounds {
        cursor: sound("CURSOR"),
        decision: sound("DECISION"),
        cancel: sound("CANCEL"),
        buzzer: sound("BUZZER"),
        ..default()
    });
    app.update();
    app
}

fn sounds(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

fn tick(app: &mut App, frame: u32) {
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

fn press(app: &mut App, keys: &[KeyCode]) {
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.reset_all();
    for key in keys {
        input.press(*key);
    }
    let frame = app.world().resource::<GameFrames>().frame + 1;
    tick(app, frame);
}

#[test]
fn commands_members_and_end_game_wrap_and_support_page_keys_without_false_sounds() {
    for (mode, count) in [(0, 5), (1, 4), (2, 2)] {
        let mut app = fixture(mode, 0);
        for (key, index, audible) in [
            (KeyCode::ArrowUp, count - 1, true),
            (KeyCode::ArrowDown, 0, true),
            (KeyCode::PageDown, count - 1, true),
            (KeyCode::PageDown, count - 1, false),
            (KeyCode::PageUp, 0, true),
            (KeyCode::PageUp, 0, false),
            (KeyCode::ArrowRight, 0, false),
            (KeyCode::ArrowLeft, 0, false),
        ] {
            press(&mut app, &[key]);
            assert_eq!(selected(&app), index, "mode {mode}, {key:?}");
            assert_eq!(
                sounds(&mut app),
                vec![AudioRequest::se("CURSOR", 100, 100).unwrap(); usize::from(audible)]
            );
        }
    }
}

#[test]
fn held_arrows_repeat_first_at_twenty_four_then_every_four_logical_frames() {
    for (mode, count) in [(0, 5), (1, 4), (2, 2)] {
        let mut app = fixture(mode, 0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        let mut moves = 0;
        for frame in 1..=64 {
            tick(&mut app, frame);
            let moving = frame == 1 || (frame >= 24 && frame % 4 == 0);
            moves += usize::from(moving);
            assert_eq!(selected(&app), moves % count, "mode {mode}, frame {frame}");
            assert_eq!(sounds(&mut app).len(), usize::from(moving));
        }
    }
}

#[test]
fn held_navigation_has_the_same_steps_and_sounds_at_every_render_rate() {
    for (mode, count) in [(0, 5), (1, 4), (2, 2)] {
        for fps in [15, 30, 60, 120, 144] {
            let mut app = fixture(mode, 0);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ArrowDown);
            let mut heard = Vec::new();
            for rendered in 1..=fps {
                tick(&mut app, rendered * 60 / fps);
                heard.extend(sounds(&mut app));
            }
            assert_eq!(selected(&app), 11 % count, "mode {mode}, {fps} FPS");
            assert_eq!(
                heard,
                vec![AudioRequest::se("CURSOR", 100, 100).unwrap(); 11]
            );
        }
    }
}

#[test]
fn navigation_precedes_decision_for_all_three_windows() {
    for (mode, expected) in [
        (0, MenuScreen::EndGame { cursor: 1 }),
        (
            1,
            MenuScreen::Equip {
                member: 3,
                slot: 0,
                picking: None,
            },
        ),
    ] {
        let mut app = fixture(mode, 0);
        press(&mut app, &[KeyCode::ArrowUp, KeyCode::Enter]);
        assert_eq!(app.world().resource::<MenuState>().screen, expected);
        assert_eq!(
            sounds(&mut app),
            ["CURSOR", "DECISION"].map(|name| AudioRequest::se(name, 100, 100).unwrap())
        );
    }
    let mut app = fixture(2, 1);
    press(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]);
    assert!(app.world().resource::<TitleActive>().0);
    assert!(!app.world().resource::<MenuOpen>().0);
    assert_eq!(
        sounds(&mut app),
        ["CURSOR", "DECISION"].map(|name| AudioRequest::se(name, 100, 100).unwrap())
    );
}

#[test]
fn navigation_precedes_cancel_but_cancel_still_wins_over_decision() {
    for mode in 0..3 {
        let mut app = fixture(mode, 0);
        press(
            &mut app,
            &[KeyCode::ArrowDown, KeyCode::Escape, KeyCode::Enter],
        );
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::Command
        );
        assert_eq!(app.world().resource::<MenuOpen>().0, mode != 0);
        assert!(!app.world().resource::<TitleActive>().0);
        assert_eq!(
            app.world().resource::<MenuState>().cursor,
            if mode == 0 { 1 } else { 2 }
        );
        assert_eq!(
            sounds(&mut app),
            ["CURSOR", "CANCEL"].map(|name| AudioRequest::se(name, 100, 100).unwrap())
        );
    }
}

#[test]
fn simultaneous_directions_use_original_order_and_single_member_arrows_remain_audible() {
    for mode in 0..3 {
        let mut app = fixture(mode, 0);
        press(
            &mut app,
            &[
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
                KeyCode::PageDown,
                KeyCode::PageUp,
            ],
        );
        assert_eq!(selected(&app), 0);
        assert_eq!(
            sounds(&mut app),
            vec![AudioRequest::se("CURSOR", 100, 100).unwrap(); 4]
        );
    }
    let mut app = fixture(1, 0);
    app.world_mut().resource_mut::<Party>().restore(vec![1]);
    press(
        &mut app,
        &[
            KeyCode::ArrowDown,
            KeyCode::ArrowUp,
            KeyCode::PageDown,
            KeyCode::PageUp,
        ],
    );
    assert_eq!(selected(&app), 0);
    assert_eq!(
        sounds(&mut app),
        vec![AudioRequest::se("CURSOR", 100, 100).unwrap(); 2]
    );
    app.world_mut().resource_mut::<Party>().restore(Vec::new());
    press(
        &mut app,
        &[
            KeyCode::ArrowDown,
            KeyCode::ArrowUp,
            KeyCode::PageDown,
            KeyCode::PageUp,
        ],
    );
    assert_eq!(selected(&app), 0);
    assert!(sounds(&mut app).is_empty());
}

#[test]
fn holding_before_opening_preserves_the_global_repeat_phase_without_a_hidden_move() {
    for mode in 0..3 {
        let mut app = fixture(mode, 0);
        app.world_mut().resource_mut::<MenuOpen>().0 = false;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        for frame in 1..=24 {
            if frame == 11 {
                app.world_mut().resource_mut::<MenuOpen>().0 = true;
            }
            tick(&mut app, frame);
            assert_eq!(selected(&app), usize::from(frame == 24));
            assert_eq!(sounds(&mut app).len(), usize::from(frame == 24));
        }
    }
}

#[test]
fn an_active_transition_blocks_commands_cancel_and_navigation_in_every_window() {
    for mode in 0..3 {
        let mut app = fixture(mode, 0);
        let mut transition = crate::transitions::Transition::default();
        transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
        app.insert_resource(transition);
        for _ in 0..35 {
            press(
                &mut app,
                &[
                    KeyCode::ArrowDown,
                    KeyCode::Enter,
                    KeyCode::Escape,
                    KeyCode::KeyS,
                ],
            );
            assert_eq!(app.world().resource::<MenuState>().screen, screen(mode, 0));
            assert!(app.world().resource::<MenuOpen>().0);
            assert!(!app.world().resource::<TitleActive>().0);
            assert!(!app.world().resource::<SaveFiles>().active());
            assert!(sounds(&mut app).is_empty());
        }
        app.world_mut()
            .resource_mut::<crate::transitions::Transition>()
            .clear();
        press(&mut app, &[]);
        assert_eq!(selected(&app), 0);
        press(&mut app, &[KeyCode::ArrowDown]);
        assert_eq!(selected(&app), 1);
    }
}

#[test]
fn a_file_selector_keeps_the_hold_phase_without_replaying_suspended_moves() {
    let mut app = fixture(0, 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    for frame in 1..=32 {
        if frame == 10 {
            app.world_mut().resource_mut::<SaveFiles>().request();
        }
        if frame == 30 {
            app.insert_resource(SaveFiles::default());
        }
        tick(&mut app, frame);
        assert_eq!(selected(&app), if frame == 32 { 2 } else { 1 });
        assert_eq!(
            sounds(&mut app).len(),
            usize::from(frame == 1 || frame == 32)
        );
    }
}
