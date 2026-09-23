use super::*;

#[test]
fn disabled_accept_buzzes_without_closing_or_leaving_a_result() {
    let mut app = app(2, 5);
    app.world_mut().resource_mut::<Choice>().disabled = vec![0];
    app.world_mut().resource_mut::<SystemSounds>().buzzer = sound("BUZZER");
    assert_eq!(press(&mut app, &[KeyCode::Enter]), expected(&["BUZZER"]));
    let choice = app.world().resource::<Choice>();
    assert!(choice.active());
    assert_eq!(choice.result, None);
    assert_eq!(choice.cursor, 0);
    assert_eq!(
        press(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]),
        expected(&["CURSOR", "DECISION"])
    );
    assert_eq!(app.world().resource::<Choice>().result, Some(1));
}

#[test]
fn disabled_accept_does_not_block_cancel_and_does_not_leak_into_another_prompt() {
    let mut app = app(2, 5);
    app.world_mut().resource_mut::<Choice>().disabled = vec![0];
    assert_eq!(press(&mut app, &[KeyCode::Escape]), expected(&["CANCEL"]));
    assert_eq!(app.world().resource::<Choice>().result, Some(4));
    app.world_mut()
        .resource_mut::<Choice>()
        .open(vec!["Next".into()], 0, 0);
    assert_eq!(press(&mut app, &[KeyCode::Enter]), expected(&["DECISION"]));
    assert_eq!(app.world().resource::<Choice>().result, Some(0));
}
