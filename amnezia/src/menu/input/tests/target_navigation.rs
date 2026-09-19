use super::*;

fn target(skill: bool, cursor: usize) -> App {
    let screen = if skill {
        MenuScreen::SkillTarget {
            member: 2,
            skill_id: 2,
            cursor,
        }
    } else {
        MenuScreen::ItemTarget {
            item_id: testkit::ITEM_HERB,
            cursor,
        }
    };
    let mut app = app_on(0, screen);
    let mut data = app.world_mut().resource_mut::<GameData>();
    for id in 2..=4 {
        let mut actor = data.actors[0].clone();
        actor.id = id;
        data.actors.push(actor);
    }
    data.skills.push(testkit::heal_skill(2, "Gyógyítás", 8, 20));
    app.world_mut()
        .resource_mut::<Party>()
        .restore(vec![1, 2, 3, 4]);
    app.insert_resource(SystemSounds {
        cursor: SoundDef {
            name: "CURSOR".into(),
            volume: 100,
            tempo: 100,
            ..default()
        },
        ..default()
    });
    app.update();
    app
}

fn cursor(app: &App) -> usize {
    match app.world().resource::<MenuState>().screen {
        MenuScreen::ItemTarget { cursor, .. } | MenuScreen::SkillTarget { cursor, .. } => cursor,
        screen => panic!("unexpected target screen {screen:?}"),
    }
}

fn heard(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

#[test]
fn both_target_types_wrap_at_the_top_and_bottom() {
    for skill in [false, true] {
        for (start, key, expected) in [(0, KeyCode::ArrowUp, 3), (3, KeyCode::ArrowDown, 0)] {
            let mut app = target(skill, start);
            press_frame(&mut app, key);
            assert_eq!(cursor(&app), expected, "{skill} {key:?}");
            assert_eq!(
                heard(&mut app),
                [AudioRequest::se("CURSOR", 100, 100).unwrap()]
            );
        }
    }
}

#[test]
fn target_page_keys_jump_to_the_first_or_last_actor_without_wrapping() {
    for skill in [false, true] {
        let mut app = target(skill, 1);
        for (key, expected, sounds) in [
            (KeyCode::PageDown, 3, 1),
            (KeyCode::PageDown, 3, 0),
            (KeyCode::PageUp, 0, 1),
            (KeyCode::PageUp, 0, 0),
        ] {
            press_frame(&mut app, key);
            assert_eq!(cursor(&app), expected);
            assert_eq!(heard(&mut app).len(), sounds);
        }
    }
}

#[test]
fn whole_party_and_self_targets_ignore_all_navigation_without_cursor_sounds() {
    for (skill, scope) in [(false, 1), (true, 2), (true, 4)] {
        let mut app = target(skill, 1);
        let mut data = app.world_mut().resource_mut::<GameData>();
        if skill {
            data.skills[1].scope = scope;
        } else {
            data.items[0].scope = scope;
        }
        for key in [
            KeyCode::ArrowDown,
            KeyCode::ArrowUp,
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::PageDown,
            KeyCode::PageUp,
        ] {
            press_frame(&mut app, key);
            assert_eq!(cursor(&app), 1);
            assert!(heard(&mut app).is_empty());
        }
    }
}

#[test]
fn held_down_repeats_on_frame_twenty_four_and_every_four_frames_including_wrap() {
    for skill in [false, true] {
        let mut app = target(skill, 0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        for frame in 1..=40 {
            app.world_mut()
                .resource_mut::<crate::timing::GameFrames>()
                .frame = frame;
            app.update();
            let moves = 1 + if frame >= 24 { 1 + (frame - 24) / 4 } else { 0 };
            assert_eq!(cursor(&app), moves as usize % 4, "{skill} frame {frame}");
            assert_eq!(
                heard(&mut app).len(),
                usize::from(frame == 1 || (frame >= 24 && frame % 4 == 0))
            );
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
        }
    }
}

#[test]
fn a_paused_target_keeps_the_held_phase_without_replaying_the_paused_moves() {
    let mut app = target(false, 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 1;
    app.update();
    assert_eq!(cursor(&app), 1);
    assert_eq!(heard(&mut app).len(), 1);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 1, IVec2::ZERO);
    app.insert_resource(transition);
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 27;
    app.update();
    assert_eq!(cursor(&app), 1);
    assert!(heard(&mut app).is_empty());
    app.insert_resource(crate::transitions::Transition::default());
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 28;
    app.update();
    assert_eq!(cursor(&app), 2);
    assert_eq!(heard(&mut app).len(), 1);
    app.update();
    assert_eq!(cursor(&app), 2);
    assert!(heard(&mut app).is_empty());
}

#[test]
fn entering_a_target_does_not_restart_a_key_already_held_in_the_previous_window() {
    let mut app = target(true, 0);
    let screen = app.world().resource::<MenuState>().screen;
    app.world_mut().resource_mut::<MenuState>().screen = MenuScreen::Command;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    for frame in 1..=23 {
        app.world_mut()
            .resource_mut::<crate::timing::GameFrames>()
            .frame = frame;
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        heard(&mut app);
    }
    app.world_mut().resource_mut::<MenuState>().screen = screen;
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 24;
    app.update();
    assert_eq!(cursor(&app), 1);
    assert_eq!(heard(&mut app).len(), 1);
}
