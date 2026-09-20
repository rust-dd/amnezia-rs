use super::*;

#[test]
fn command_navigation_precedes_decision_and_remembers_the_selected_command() {
    let mut app = app(Phase::Command {
        cursor: 0,
        regreet: false,
    });
    press(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]);
    assert!(matches!(phase(&app), Phase::Sell { cursor: 0 }));
    assert_eq!(sounds(&mut app), ["CURSOR", "DECISION"]);
    press(&mut app, &[KeyCode::Escape]);
    assert!(matches!(
        phase(&app),
        Phase::Command {
            cursor: 1,
            regreet: true
        }
    ));
    assert_eq!(sounds(&mut app), ["CANCEL"]);
    press(&mut app, &[KeyCode::Enter, KeyCode::Escape]);
    assert!(matches!(*app.world().resource::<Screen>(), Screen::Closed));
    assert_eq!(sounds(&mut app), ["DECISION", "CANCEL"]);
}

#[test]
fn buy_selection_survives_visiting_the_command_menu_and_other_list() {
    let mut app = app(Phase::Buy { cursor: 0 });
    press(&mut app, &[KeyCode::ArrowDown]);
    press(&mut app, &[KeyCode::ArrowDown]);
    assert!(matches!(phase(&app), Phase::Buy { cursor: 2 }));
    press(&mut app, &[KeyCode::Escape]);
    press(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]);
    assert!(matches!(phase(&app), Phase::Sell { cursor: 0 }));
    press(&mut app, &[KeyCode::Escape]);
    press(&mut app, &[KeyCode::ArrowUp, KeyCode::Enter]);
    assert!(matches!(phase(&app), Phase::Buy { cursor: 2 }));
}

#[test]
fn help_clears_on_phase_changes_and_hidden_message_cursor_keeps_its_clock() {
    let mut app = app(Phase::Buy { cursor: 0 });
    let state = |app: &App| {
        let Screen::Shop(state) = app.world().resource::<Screen>() else {
            panic!()
        };
        (
            state.scene.help_id,
            state.scene.command_frame,
            state.scene.buy.cursor_frame,
        )
    };
    assert_eq!(state(&app), (1, 0, 0));
    press(&mut app, &[KeyCode::Enter]);
    assert_eq!(state(&app), (0, 1, 1));
    tick(&mut app, 10);
    assert_eq!(state(&app), (0, 11, 1));
    press(&mut app, &[KeyCode::Escape]);
    assert_eq!(state(&app), (0, 12, 1));
    tick(&mut app, 1);
    assert_eq!(state(&app), (1, 13, 2));
}

#[test]
fn held_command_navigation_repeats_at_twenty_four_then_every_four_frames() {
    let mut app = app(Phase::Command {
        cursor: 0,
        regreet: false,
    });
    press(&mut app, &[KeyCode::ArrowDown]);
    assert_eq!(sounds(&mut app), ["CURSOR"]);
    tick(&mut app, 22);
    assert!(matches!(phase(&app), Phase::Command { cursor: 1, .. }));
    assert!(sounds(&mut app).is_empty());
    tick(&mut app, 1);
    assert!(matches!(phase(&app), Phase::Command { cursor: 2, .. }));
    assert_eq!(sounds(&mut app), ["CURSOR"]);
    tick(&mut app, 4);
    assert!(matches!(phase(&app), Phase::Command { cursor: 0, .. }));
    assert_eq!(sounds(&mut app), ["CURSOR"]);
}
