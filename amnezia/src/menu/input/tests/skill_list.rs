use super::*;
use amnezia_data::Learning;

fn list(cursor: usize) -> App {
    let mut app = app_on(1, MenuScreen::SkillList { member: 0, cursor });
    let mut data = app.world_mut().resource_mut::<GameData>();
    data.skills = vec![
        testkit::skill(1, "Csapás", 1),
        testkit::heal_skill(2, "Gyógyítás", 8, 20),
        testkit::heal_skill(3, "Nagy gyógyítás", 40, 50),
        testkit::heal_skill(4, "Másik gyógyítás", 9, 20),
    ];
    data.actors[0].learnings = (1..=4)
        .map(|skill_id| Learning { level: 1, skill_id })
        .collect();
    app.world_mut().resource_mut::<Vitals>().set(1, 20, 7);
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
    app
}

fn heard(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

fn assert_cursor(app: &App, cursor: usize) {
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::SkillList { member: 0, cursor }
    );
}

#[test]
fn unaffordable_skill_cannot_open_the_target_and_only_buzzes() {
    let mut app = list(1);
    press_frame(&mut app, KeyCode::Enter);
    assert_cursor(&app, 1);
    assert_eq!(
        heard(&mut app),
        [AudioRequest::se("BUZZER", 100, 100).unwrap()]
    );
    assert_eq!(
        app.world().resource::<Vitals>().get_stored(1),
        Some((20, 7))
    );
}

#[test]
fn battle_only_skill_uses_the_original_buzzer_instead_of_decision() {
    let mut app = list(0);
    press_frame(&mut app, KeyCode::Space);
    assert_cursor(&app, 0);
    assert_eq!(
        heard(&mut app),
        [AudioRequest::se("BUZZER", 100, 100).unwrap()]
    );
}

#[test]
fn skill_list_navigates_two_columns_without_wrapping_or_paging() {
    let mut app = list(0);
    for (key, cursor, moved) in [
        (KeyCode::ArrowDown, 2, true),
        (KeyCode::ArrowRight, 3, true),
        (KeyCode::ArrowDown, 3, false),
        (KeyCode::PageUp, 3, false),
        (KeyCode::PageDown, 3, false),
        (KeyCode::ArrowLeft, 2, true),
        (KeyCode::ArrowUp, 0, true),
        (KeyCode::ArrowLeft, 0, false),
    ] {
        press_frame(&mut app, key);
        assert_cursor(&app, cursor);
        assert_eq!(
            heard(&mut app),
            if moved {
                vec![AudioRequest::se("CURSOR", 100, 100).unwrap()]
            } else {
                vec![]
            }
        );
    }
}

#[test]
fn half_cost_affordability_is_shared_with_target_entry() {
    let mut app = list(1);
    let actor = app.world().resource::<GameData>().actors[0].clone();
    app.world_mut().resource_mut::<GameData>().items =
        crate::assets::load_ron(&format!("{}/items.ron", crate::assets::asset_root()));
    app.world_mut()
        .resource_mut::<Equipment>()
        .set_slot(&actor, 1, 152);
    press_frame(&mut app, KeyCode::Enter);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::SkillTarget {
            member: 0,
            skill_id: 2,
            cursor: 0
        }
    );
    assert_eq!(
        heard(&mut app),
        [AudioRequest::se("DECISION", 100, 100).unwrap()]
    );
    assert_eq!(
        app.world().resource::<Vitals>().get_stored(1),
        Some((20, 7))
    );
}

#[test]
fn dead_silenced_and_missing_weapon_casters_cannot_open_the_target() {
    for restriction in 0..3 {
        let mut app = list(1);
        app.world_mut()
            .resource_mut::<Vitals>()
            .set(1, if restriction == 0 { 0 } else { 20 }, 37);
        if restriction == 1 {
            app.world_mut().resource_mut::<GameData>().skills[1].magical_rate = 10;
            app.world_mut()
                .resource_mut::<Vitals>()
                .set_states(1, vec![4]);
        } else if restriction == 2 {
            app.world_mut().resource_mut::<GameData>().skills[1].attributes = vec![1];
        }
        press_frame(&mut app, KeyCode::Enter);
        assert_cursor(&app, 1);
        assert_eq!(
            heard(&mut app),
            [AudioRequest::se("BUZZER", 100, 100).unwrap()],
            "restriction {restriction}"
        );
    }
}

#[test]
fn empty_skill_list_keeps_its_blank_selection_and_buzzes() {
    let mut app = list(0);
    app.world_mut().resource_mut::<GameData>().actors[0]
        .learnings
        .clear();
    press_frame(&mut app, KeyCode::ArrowDown);
    assert_cursor(&app, 0);
    assert!(heard(&mut app).is_empty());
    press_frame(&mut app, KeyCode::Enter);
    assert_cursor(&app, 0);
    assert_eq!(
        heard(&mut app),
        [AudioRequest::se("BUZZER", 100, 100).unwrap()]
    );
}

#[test]
fn list_navigation_precedes_cancel_and_cancel_precedes_confirmation() {
    let mut app = list(0);
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.press(KeyCode::ArrowRight);
    keys.press(KeyCode::Escape);
    keys.press(KeyCode::Enter);
    app.update();
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::MemberSelect {
            action: crate::menu::MemberAction::Skill,
            cursor: 0
        }
    );
    assert_eq!(
        heard(&mut app),
        [
            AudioRequest::se("CURSOR", 100, 100).unwrap(),
            AudioRequest::se("CANCEL", 100, 100).unwrap()
        ]
    );
}

#[test]
fn transition_freezes_skill_input_and_does_not_replay_elapsed_ticks() {
    let mut app = list(1);
    app.update();
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
    app.insert_resource(transition);
    for (frame, key) in [
        (1, KeyCode::Enter),
        (2, KeyCode::ArrowDown),
        (600, KeyCode::Escape),
    ] {
        app.world_mut()
            .resource_mut::<crate::timing::GameFrames>()
            .frame = frame;
        press_frame(&mut app, key);
        assert_cursor(&app, 1);
        assert!(heard(&mut app).is_empty());
        assert_eq!(
            app.world()
                .resource::<skills::List>()
                .navigation
                .cursor_frame,
            0
        );
    }
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
    assert_eq!(
        app.world()
            .resource::<skills::List>()
            .navigation
            .cursor_frame,
        0
    );
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 601;
    app.update();
    assert_eq!(
        app.world()
            .resource::<skills::List>()
            .navigation
            .cursor_frame,
        1
    );
}

#[test]
fn returning_from_a_scrolled_skill_preserves_selection_scroll_and_cursor_phase() {
    let mut app = list(20);
    let mut data = app.world_mut().resource_mut::<GameData>();
    data.skills = (1..=25)
        .map(|id| testkit::heal_skill(id, "Gyógyítás", 1, 20))
        .collect();
    data.actors[0].learnings = (1..=25)
        .map(|skill_id| Learning { level: 1, skill_id })
        .collect();
    app.update();
    assert_eq!(app.world().resource::<skills::List>().navigation.offset, 16);
    press_frame(&mut app, KeyCode::Enter);
    assert!(matches!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::SkillTarget { skill_id: 21, .. }
    ));
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 60;
    press_frame(&mut app, KeyCode::Escape);
    assert_cursor(&app, 20);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
    let nav = &app.world().resource::<skills::List>().navigation;
    assert_eq!((nav.index, nav.offset, nav.cursor_frame), (20, 16, 0));
}
