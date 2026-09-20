use super::*;
use crate::menu::equip::Scene;
use crate::timing::GameFrames;

fn screen(member: usize, slot: usize, picking: Option<usize>) -> MenuScreen {
    MenuScreen::Equip {
        member,
        slot,
        picking,
    }
}

fn prepared(slot: usize) -> App {
    let mut app = app_on(2, screen(0, slot, None));
    let mut data = app.world_mut().resource_mut::<GameData>();
    data.actors = (1..=3)
        .map(|id| {
            let mut actor = testkit::actor();
            actor.id = id;
            actor.level = id + 1;
            actor.weapon = id + 9;
            actor.fix_equipment = id == 3;
            actor
        })
        .collect();
    data.items = (10..40).map(|id| testkit::weapon(id, "Kard", id)).collect();
    app.world_mut()
        .resource_mut::<Party>()
        .restore(vec![1, 2, 3]);
    for id in 10..40 {
        app.world_mut().resource_mut::<Inventory>().add_item(id, 1);
    }
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

fn heard(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

fn press(app: &mut App, keys: &[KeyCode]) {
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.reset_all();
    for key in keys {
        input.press(*key);
    }
    app.update();
}

#[test]
fn fresh_horizontal_keys_cycle_members_and_preserve_every_slot() {
    for slot in 0..5 {
        let mut app = prepared(slot);
        for (key, member) in [
            (KeyCode::ArrowRight, 1),
            (KeyCode::ArrowRight, 2),
            (KeyCode::ArrowRight, 0),
            (KeyCode::ArrowLeft, 2),
            (KeyCode::ArrowLeft, 1),
            (KeyCode::ArrowLeft, 0),
        ] {
            press_frame(&mut app, key);
            assert_eq!(
                app.world().resource::<MenuState>().screen,
                screen(member, slot, None)
            );
            assert_eq!(
                heard(&mut app),
                [AudioRequest::se("CURSOR", 100, 100).unwrap()]
            );
        }
        assert!(app.world().resource::<Equipment>().entries().is_empty());
        for id in 10..40 {
            assert_eq!(app.world().resource::<Inventory>().count(id), 1);
        }
    }
}

#[test]
fn held_horizontal_input_switches_only_once_at_every_render_rate() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = prepared(3);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        let mut sounds = Vec::new();
        for rendered in 1..=fps {
            app.world_mut().resource_mut::<GameFrames>().frame = rendered * 60 / fps;
            app.update();
            sounds.extend(heard(&mut app));
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
        }
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(1, 3, None),
            "{fps} FPS"
        );
        assert_eq!(sounds, [AudioRequest::se("CURSOR", 100, 100).unwrap()]);
        press_frame(&mut app, KeyCode::ArrowRight);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(2, 3, None)
        );
    }
}

#[test]
fn single_member_horizontal_input_is_silent_and_does_not_restart_the_scene() {
    let mut app = prepared(2);
    app.world_mut().resource_mut::<Party>().restore(vec![1]);
    app.world_mut().resource_mut::<Scene>().slot_frame = 9;
    for key in [KeyCode::ArrowLeft, KeyCode::ArrowRight] {
        press_frame(&mut app, key);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(0, 2, None)
        );
        assert_eq!(app.world().resource::<Scene>().slot_frame, 9);
        assert!(heard(&mut app).is_empty());
    }
}

#[test]
fn cancel_then_confirm_then_right_have_priority_over_actor_switching() {
    for (keys, expected, sound) in [
        (
            vec![KeyCode::Escape, KeyCode::Enter, KeyCode::ArrowRight],
            MenuScreen::Command,
            "CANCEL",
        ),
        (
            vec![KeyCode::Enter, KeyCode::ArrowRight],
            screen(0, 0, Some(0)),
            "DECISION",
        ),
        (
            vec![KeyCode::ArrowLeft, KeyCode::ArrowRight],
            screen(1, 0, None),
            "CURSOR",
        ),
    ] {
        let mut app = prepared(0);
        press(&mut app, &keys);
        assert_eq!(app.world().resource::<MenuState>().screen, expected);
        assert_eq!(
            heard(&mut app),
            [AudioRequest::se(sound, 100, 100).unwrap()]
        );
    }
    let mut app = prepared(0);
    app.world_mut().resource_mut::<MenuState>().screen = screen(2, 0, None);
    press(&mut app, &[KeyCode::Space, KeyCode::ArrowRight]);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(2, 0, None)
    );
    assert_eq!(
        heard(&mut app),
        [AudioRequest::se("BUZZER", 100, 100).unwrap()]
    );
}

#[test]
fn item_picker_horizontal_keys_move_items_without_switching_members() {
    let mut app = prepared(0);
    press_frame(&mut app, KeyCode::Enter);
    heard(&mut app);
    for (key, cursor) in [(KeyCode::ArrowRight, 1), (KeyCode::ArrowLeft, 0)] {
        press_frame(&mut app, key);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(0, 0, Some(cursor))
        );
        assert_eq!(
            heard(&mut app),
            [AudioRequest::se("CURSOR", 100, 100).unwrap()]
        );
    }
}

#[test]
fn switching_replaces_all_list_banks_and_preview_before_the_new_actor_is_drawn() {
    let mut app = prepared(0);
    let mut scene = app.world_mut().resource_mut::<Scene>();
    scene.slot_frame = 17;
    scene.preview = Some([999; 4]);
    for nav in &mut scene.lists {
        nav.refresh(10, 31);
        nav.tick([true, false, false, false], true);
        nav.tick([false; 4], true);
        assert_eq!(nav.offset, 4);
    }
    press_frame(&mut app, KeyCode::ArrowRight);
    let scene = app.world().resource::<Scene>();
    assert_eq!(
        (scene.member, scene.current, scene.preview, scene.help_id),
        (Some(1), [45, 20, 17, 14], None, 11)
    );
    assert_eq!(scene.slot_frame, 0);
    for (slot, nav) in scene.lists.iter().enumerate() {
        assert_eq!(
            (nav.index, nav.offset, nav.cursor_frame, nav.arrow_frame),
            (0, 0, 0, 0)
        );
        assert_eq!(nav.count(), if slot == 0 { 31 } else { 1 });
        assert_eq!(nav.arrows, [false; 2]);
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.world_mut().resource_mut::<GameFrames>().frame = 10;
    app.update();
    assert!(
        app.world()
            .resource::<Scene>()
            .lists
            .iter()
            .all(|nav| nav.offset == 0)
    );
    assert_eq!(app.world().resource::<Scene>().lists[0].arrow_frame, 10);
}

#[test]
fn slot_navigation_precedes_actor_switching_on_the_same_frame() {
    let mut app = prepared(2);
    press(&mut app, &[KeyCode::ArrowDown, KeyCode::ArrowRight]);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(1, 3, None)
    );
    assert_eq!(
        heard(&mut app),
        vec![AudioRequest::se("CURSOR", 100, 100).unwrap(); 2]
    );
}

#[test]
fn an_arrow_pressed_during_a_transition_is_not_replayed_afterwards() {
    let mut app = prepared(1);
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
    app.insert_resource(transition);
    press_frame(&mut app, KeyCode::ArrowRight);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(0, 1, None)
    );
    assert!(heard(&mut app).is_empty());
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut().resource_mut::<GameFrames>().frame = 60;
    app.update();
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(0, 1, None)
    );
    assert!(heard(&mut app).is_empty());
    press_frame(&mut app, KeyCode::ArrowRight);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(1, 1, None)
    );
}
