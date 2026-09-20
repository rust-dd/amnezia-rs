use super::*;

fn count(app: &App) -> u32 {
    let Phase::Number(number) = phase(app) else {
        panic!("quantity window closed unexpectedly");
    };
    number.count
}

#[test]
fn quantity_uses_original_axes_limits_and_direction_priority() {
    for (start, keys, expected) in [
        (20, vec![KeyCode::ArrowRight], 21),
        (20, vec![KeyCode::ArrowLeft], 19),
        (20, vec![KeyCode::ArrowUp], 30),
        (20, vec![KeyCode::ArrowDown], 10),
        (95, vec![KeyCode::ArrowUp], 99),
        (5, vec![KeyCode::ArrowDown], 1),
        (99, vec![KeyCode::ArrowRight], 99),
        (1, vec![KeyCode::ArrowLeft], 1),
        (20, vec![KeyCode::ArrowDown, KeyCode::ArrowRight], 21),
        (99, vec![KeyCode::ArrowRight, KeyCode::ArrowLeft], 98),
    ] {
        let mut app = app(number(Mode::Buy, start, 99, 0));
        press(&mut app, &keys);
        assert_eq!(count(&app), expected, "{keys:?}, from {start}");
        assert_eq!(sounds(&mut app).len(), usize::from(start != expected));
    }
}

#[test]
fn quantity_navigation_precedes_confirm_and_cancel_with_both_sounds() {
    for cancel in [false, true] {
        let mut app = app(number(Mode::Buy, 1, 99, 2));
        app.world_mut().resource_mut::<GameData>().items[0].price = 10;
        let mut keys = vec![KeyCode::ArrowUp, KeyCode::Enter];
        if cancel {
            keys.push(KeyCode::Escape);
        }
        press(&mut app, &keys);
        assert_eq!(
            sounds(&mut app),
            ["CURSOR", if cancel { "CANCEL" } else { "DECISION" }]
        );
        assert_eq!(
            app.world().resource::<Inventory>().count(1),
            if cancel { 0 } else { 11 }
        );
        assert_eq!(app.world().resource::<ShopOutcome>().transacted, !cancel);
        assert!(!matches!(phase(&app), Phase::Number(_)));
    }
}

#[test]
fn held_quantity_repeats_on_the_shared_twenty_four_four_clock_at_every_fps() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = app(number(Mode::Buy, 1, 99, 0));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        let mut previous = 0;
        let mut heard = Vec::new();
        for render in 1..=fps {
            let elapsed = render * 60 / fps;
            tick(&mut app, elapsed - previous);
            previous = elapsed;
            heard.extend(sounds(&mut app));
        }
        assert_eq!(count(&app), 12, "{fps} FPS");
        assert_eq!(heard, vec!["CURSOR"; 11]);
    }
}

#[test]
fn async_terminal_frame_ignores_quantity_keys_without_replaying_them() {
    let mut app = app(number(Mode::Buy, 1, 99, 0));
    app.world_mut().resource_mut::<SceneWait>().0 = true;
    press(&mut app, &[KeyCode::ArrowUp, KeyCode::Enter]);
    assert_eq!(count(&app), 1);
    assert!(sounds(&mut app).is_empty());
    assert!(!app.world().resource::<ShopOutcome>().transacted);
    app.world_mut().resource_mut::<SceneWait>().0 = false;
    tick(&mut app, 1);
    assert_eq!(count(&app), 1);
    press(&mut app, &[KeyCode::ArrowUp, KeyCode::Enter]);
    assert!(app.world().resource::<ShopOutcome>().transacted);
}
