use super::*;

fn target(count: u32) -> App {
    let mut app = app_on(0, MenuScreen::ItemList { cursor: 0 });
    app.world_mut()
        .resource_mut::<Inventory>()
        .add_item(testkit::ITEM_HERB, count);
    app.world_mut().resource_mut::<Vitals>().set(1, 7, 5);
    press_frame(&mut app, KeyCode::Enter);
    app
}

#[test]
fn using_the_last_item_keeps_the_target_open_until_cancelled() {
    let mut app = target(1);
    press_frame(&mut app, KeyCode::Enter);
    assert_eq!(
        app.world()
            .resource::<Inventory>()
            .count(testkit::ITEM_HERB),
        0
    );
    assert_eq!(
        app.world().resource::<Vitals>().get_stored(1),
        Some((33, 5))
    );
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::ItemTarget {
            item_id: testkit::ITEM_HERB,
            cursor: 0
        },
    );
    press_frame(&mut app, KeyCode::Escape);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::ItemList { cursor: 0 }
    );
}

#[test]
fn using_an_ineffective_item_plays_the_buzzer_without_a_decision_sound() {
    let mut app = target(2);
    app.world_mut().resource_mut::<Vitals>().set(1, 63, 37);
    let mut sounds = SystemSounds::default();
    sounds.decision.name = "DECISION".into();
    sounds.buzzer.name = "BUZZER".into();
    app.insert_resource(sounds);
    press_frame(&mut app, KeyCode::Enter);
    let heard = app
        .world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .filter_map(|audio| match audio {
            AudioRequest::Sound { name, .. } => Some(name),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(heard, ["BUZZER"]);
    assert_eq!(
        app.world()
            .resource::<Inventory>()
            .count(testkit::ITEM_HERB),
        2
    );
    assert_eq!(
        app.world().resource::<Vitals>().get_stored(1),
        Some((63, 37))
    );
}

#[test]
fn item_use_precedes_cancellation_when_both_are_pressed_together() {
    let mut app = target(2);
    app.insert_resource(sounds());
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    keys.press(KeyCode::Escape);
    keys.press(KeyCode::Enter);
    app.update();
    assert_eq!(
        app.world().resource::<Vitals>().get_stored(1),
        Some((33, 5))
    );
    assert_eq!(
        app.world()
            .resource::<Inventory>()
            .count(testkit::ITEM_HERB),
        1
    );
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::ItemList { cursor: 0 }
    );
    assert_eq!(
        heard(&mut app),
        [
            AudioRequest::se("Item1", 90, 130).unwrap(),
            AudioRequest::se("CANCEL", 100, 100).unwrap()
        ]
    );
}

fn sounds() -> SystemSounds {
    SystemSounds {
        item: SoundDef {
            name: "Item1".into(),
            volume: 90,
            tempo: 130,
            ..default()
        },
        buzzer: SoundDef {
            name: "BUZZER".into(),
            volume: 100,
            tempo: 100,
            ..default()
        },
        cancel: SoundDef {
            name: "CANCEL".into(),
            volume: 100,
            tempo: 100,
            ..default()
        },
        decision: SoundDef {
            name: "DECISION".into(),
            volume: 100,
            tempo: 100,
            ..default()
        },
        ..default()
    }
}

fn heard(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

#[test]
fn successive_successful_uses_play_only_the_item_sound_with_original_volume_and_tempo() {
    let mut app = target(2);
    app.insert_resource(sounds());
    for key in [KeyCode::Enter, KeyCode::Space] {
        press_frame(&mut app, key);
        assert_eq!(
            heard(&mut app),
            [AudioRequest::se("Item1", 90, 130).unwrap()]
        );
        assert!(matches!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::ItemTarget { .. }
        ));
    }
    assert_eq!(
        app.world()
            .resource::<Inventory>()
            .count(testkit::ITEM_HERB),
        0
    );
    let vitals = app.world().resource::<Vitals>().get_stored(1);
    press_frame(&mut app, KeyCode::Enter);
    assert_eq!(
        heard(&mut app),
        [AudioRequest::se("BUZZER", 100, 100).unwrap()]
    );
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), vitals);
}

#[test]
fn a_disabled_item_sound_stays_silent_without_falling_back_to_decision() {
    let mut app = target(2);
    let mut sounds = sounds();
    sounds.item.name = "(OFF)".into();
    app.insert_resource(sounds);
    press_frame(&mut app, KeyCode::Enter);
    assert!(heard(&mut app).is_empty());
    assert_eq!(
        app.world()
            .resource::<Inventory>()
            .count(testkit::ITEM_HERB),
        1
    );
}

#[test]
fn a_scene_transition_blocks_target_use_and_cancel_together() {
    let mut app = target(2);
    app.insert_resource(sounds());
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
    app.insert_resource(transition);
    for key in [KeyCode::Enter, KeyCode::Space, KeyCode::Escape] {
        press_frame(&mut app, key);
        assert!(heard(&mut app).is_empty());
        assert_eq!(
            app.world()
                .resource::<Inventory>()
                .count(testkit::ITEM_HERB),
            2
        );
        assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((7, 5)));
        assert!(matches!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::ItemTarget { .. }
        ));
    }
}

#[test]
fn the_audio_plugin_loads_the_original_field_item_sound() {
    let mut app = App::new();
    app.add_plugins(crate::audio::AudioPlugin);
    assert_eq!(
        app.world().resource::<SystemSounds>().item,
        SoundDef {
            name: "Item1".into(),
            volume: 90,
            tempo: 100,
            balance: 50
        }
    );
}
