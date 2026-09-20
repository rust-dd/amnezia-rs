use super::*;
use crate::menu::MemberAction;

fn entered(command: usize) -> App {
    let mut app = app_on(command, MenuScreen::Command);
    let mut actor = testkit::actor();
    actor.id = 2;
    app.world_mut()
        .resource_mut::<GameData>()
        .actors
        .push(actor);
    app.world_mut().resource_mut::<Party>().restore(vec![1, 2]);
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
    press_frame(&mut app, KeyCode::Enter);
    press_frame(&mut app, KeyCode::ArrowDown);
    press_frame(&mut app, KeyCode::Enter);
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .clear();
    app
}

fn heard(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

fn check_return(command: usize) {
    let mut app = entered(command);
    press_frame(&mut app, KeyCode::Escape);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::Command
    );
    assert!(app.world().resource::<MenuOpen>().0);
    assert_eq!(app.world().resource::<MenuState>().cursor, command);
    assert_eq!(
        heard(&mut app),
        [AudioRequest::se("CANCEL", 100, 100).unwrap()]
    );
    press_frame(&mut app, KeyCode::Enter);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::MemberSelect {
            action: if command == 1 {
                MemberAction::Skill
            } else {
                MemberAction::Equip
            },
            cursor: 0,
        }
    );
    assert_eq!(
        heard(&mut app),
        [AudioRequest::se("DECISION", 100, 100).unwrap()]
    );
}

#[test]
fn skill_scene_returns_to_its_command_and_reselects_from_the_first_member() {
    check_return(1);
}

#[test]
fn equipment_scene_returns_to_its_command_and_reselects_from_the_first_member() {
    check_return(2);
}

#[test]
fn simultaneous_confirm_cannot_reenter_a_scene_on_the_cancel_frame() {
    for command in [1, 2] {
        let mut app = entered(command);
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::Escape);
        keys.press(KeyCode::Enter);
        app.update();
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::Command
        );
        assert_eq!(
            heard(&mut app),
            [AudioRequest::se("CANCEL", 100, 100).unwrap()]
        );
    }
}

#[test]
fn leaving_a_scene_then_the_command_closes_the_menu_in_two_cancels() {
    for command in [1, 2] {
        let mut app = entered(command);
        for _ in 0..2 {
            press_frame(&mut app, KeyCode::Escape);
        }
        assert!(!app.world().resource::<MenuOpen>().0);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::Command
        );
        assert_eq!(app.world().resource::<MenuState>().cursor, command);
        assert_eq!(
            heard(&mut app),
            vec![AudioRequest::se("CANCEL", 100, 100).unwrap(); 2]
        );
    }
}
