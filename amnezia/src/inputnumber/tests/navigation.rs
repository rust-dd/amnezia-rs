use super::*;

fn heard(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
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

fn cursor(count: usize) -> Vec<AudioRequest> {
    vec![AudioRequest::se("CURSOR", 100, 100).unwrap(); count]
}

#[test]
fn all_original_number_commands_start_at_the_units_digit() {
    let mut count = 0;
    for id in [102, 106, 182, 184, 189, 191, 194, 196, 198, 247, 256] {
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/../assets/maps/map_{id:04}.ron",
            env!("CARGO_MANIFEST_DIR")
        ));
        for command in map
            .events
            .iter()
            .flat_map(|event| &event.pages)
            .flat_map(|page| &page.commands)
            .filter(|command| command.code == 10150)
        {
            let mut number = InputNumber::default();
            let digits = command.params[0] as u32;
            assert!((1..=4).contains(&digits));
            number.open(digits, command.params[1] as u32);
            assert_eq!(
                number.cursor,
                digits as usize - 1,
                "map {id}, {:?}",
                command.params
            );
            assert_eq!(number.value, 0);
            assert_eq!(number.slots, vec![0; digits as usize]);
            count += 1;
        }
    }
    assert_eq!(count, 33);
}

#[test]
fn held_arrows_repeat_on_tick_24_then_every_four_ticks_at_all_render_rates() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = headless();
        app.world_mut().resource_mut::<InputNumber>().open(1, 7);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        let mut sounds = Vec::new();
        for frame in 1..=fps {
            let tick = frame * 60 / fps;
            app.world_mut().resource_mut::<GameFrames>().frame = tick;
            app.update();
            let moves = 1 + if tick < 24 { 0 } else { 1 + (tick - 24) / 4 };
            assert_eq!(
                app.world().resource::<InputNumber>().value,
                (10 - moves as i64 % 10) % 10,
                "{fps} FPS, tick {tick}"
            );
            sounds.extend(heard(&mut app));
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
        }
        assert_eq!(sounds, cursor(11), "{fps} FPS");
    }
}

#[test]
fn simultaneous_directions_play_three_cursor_sounds_before_decision() {
    let mut app = headless();
    app.world_mut().resource_mut::<InputNumber>().open(3, 8);
    let sounds = press(
        &mut app,
        &[
            KeyCode::ArrowDown,
            KeyCode::ArrowUp,
            KeyCode::ArrowRight,
            KeyCode::ArrowLeft,
            KeyCode::Enter,
            KeyCode::Space,
        ],
    );
    let mut expected = cursor(3);
    expected.push(AudioRequest::se("DECISION", 100, 100).unwrap());
    assert_eq!(sounds, expected);
    let number = app.world().resource::<InputNumber>();
    assert_eq!(number.result, Some(0));
    assert_eq!(number.cursor, 2);
    assert!(!number.active());
}

#[test]
fn one_digit_right_is_silent_but_left_and_vertical_arrows_are_not() {
    let mut app = headless();
    app.world_mut().resource_mut::<InputNumber>().open(1, 8);
    assert!(press(&mut app, &[KeyCode::ArrowRight]).is_empty());
    assert_eq!(press(&mut app, &[KeyCode::ArrowLeft]), cursor(1));
    assert_eq!(
        press(&mut app, &[KeyCode::ArrowUp, KeyCode::ArrowDown]),
        cursor(1)
    );
    assert_eq!(app.world().resource::<InputNumber>().value, 0);
    assert_eq!(app.world().resource::<InputNumber>().cursor, 0);
}

#[test]
fn opening_and_resuming_do_not_retrigger_held_arrows_or_replay_blocked_moves() {
    let mut app = headless();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    for frame in 1..=64 {
        if frame == 31 {
            app.world_mut().resource_mut::<InputNumber>().open(1, 8);
        }
        app.world_mut().resource_mut::<crate::menu::MenuOpen>().0 = (33..=60).contains(&frame);
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
        let expected = if frame < 32 {
            0
        } else if frame < 64 {
            9
        } else {
            8
        };
        assert_eq!(
            app.world().resource::<InputNumber>().value,
            expected,
            "tick {frame}"
        );
        assert_eq!(
            heard(&mut app),
            cursor(usize::from(matches!(frame, 32 | 64))),
            "tick {frame}"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }
}

#[test]
fn transitions_and_open_menus_block_confirmation_cancel_and_digit_edits() {
    for menu in [false, true] {
        let mut app = headless();
        app.world_mut().resource_mut::<InputNumber>().open(2, 8);
        if menu {
            app.world_mut().resource_mut::<crate::menu::MenuOpen>().0 = true;
        } else {
            let mut transition = crate::transitions::Transition::default();
            transition.start_for(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO, 6);
            app.insert_resource(transition);
        }
        assert!(
            press(
                &mut app,
                &[
                    KeyCode::ArrowUp,
                    KeyCode::ArrowRight,
                    KeyCode::Enter,
                    KeyCode::Space,
                    KeyCode::Escape
                ]
            )
            .is_empty()
        );
        let number = app.world().resource::<InputNumber>();
        assert!(number.active());
        assert_eq!((number.value, number.cursor, number.result), (0, 1, None));
        app.world_mut().resource_mut::<crate::menu::MenuOpen>().0 = false;
        app.insert_resource(crate::transitions::Transition::default());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert!(app.world().resource::<InputNumber>().active());
        assert!(heard(&mut app).is_empty());
    }
}

#[test]
fn render_only_updates_do_not_repeat_and_reopening_resets_the_value_and_cursor() {
    let mut app = headless();
    app.world_mut().resource_mut::<InputNumber>().open(4, 8);
    assert_eq!(press(&mut app, &[KeyCode::ArrowUp]), cursor(1));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    for _ in 0..40 {
        app.update();
        assert_eq!(app.world().resource::<InputNumber>().value, 1);
        assert!(heard(&mut app).is_empty());
    }
    press(&mut app, &[KeyCode::Enter]);
    app.world_mut().resource_mut::<InputNumber>().open(2, 20);
    let number = app.world().resource::<InputNumber>();
    assert_eq!((number.value, number.cursor, number.result), (0, 1, None));
    assert_eq!(number.slots, [0, 0]);
}

#[test]
fn cancel_keeps_the_number_open_and_confirmation_uses_the_updated_units_digit() {
    let mut app = headless();
    app.world_mut().resource_mut::<InputNumber>().open(2, 20);
    assert_eq!(
        press(&mut app, &[KeyCode::ArrowUp, KeyCode::Escape]),
        cursor(1)
    );
    assert!(app.world().resource::<InputNumber>().active());
    let sounds = press(
        &mut app,
        &[KeyCode::ArrowUp, KeyCode::Enter, KeyCode::Space],
    );
    assert_eq!(
        sounds,
        [
            AudioRequest::se("CURSOR", 100, 100).unwrap(),
            AudioRequest::se("DECISION", 100, 100).unwrap()
        ]
    );
    assert_eq!(app.world().resource::<InputNumber>().result, Some(2));
}
