use super::*;
use crate::menu::MemberAction;

fn with_sounds(cursor: usize, screen: MenuScreen) -> App {
    let mut app = app_on(cursor, screen);
    let sound = |name: &str| SoundDef {
        name: name.into(),
        volume: 100,
        tempo: 100,
        ..default()
    };
    app.insert_resource(SystemSounds {
        cursor: sound("cursor"),
        decision: sound("decision"),
        cancel: sound("cancel"),
        buzzer: sound("buzzer"),
    });
    app
}

fn sounds(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

#[test]
fn empty_party_cannot_open_items_skills_or_equipment() {
    for cursor in 0..3 {
        let mut app = app_on(cursor, MenuScreen::Command);
        app.world_mut().resource_mut::<Party>().restore(Vec::new());
        confirm(&mut app, KeyCode::Enter);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::Command
        );
    }
}

#[test]
fn disabled_commands_play_one_buzzer_and_no_decision() {
    for cursor in 0..4 {
        let mut app = with_sounds(cursor, MenuScreen::Command);
        app.world_mut().resource_mut::<Party>().restore(Vec::new());
        confirm(&mut app, KeyCode::Enter);
        assert_eq!(
            sounds(&mut app),
            [AudioRequest::play_sound("buzzer", &[100, 100])]
        );
        assert!(!app.world().resource::<SaveRequest>().0);
    }
}

#[test]
fn right_arrow_does_not_open_a_non_original_status_shortcut() {
    let mut app = with_sounds(0, MenuScreen::Command);
    confirm(&mut app, KeyCode::ArrowRight);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::Command
    );
    assert!(sounds(&mut app).is_empty());
}

#[test]
fn enabled_commands_play_one_decision_and_enter_their_flows() {
    for cursor in 0..5 {
        let mut app = with_sounds(cursor, MenuScreen::Command);
        app.world_mut().resource_mut::<SaveAccess>().0 = true;
        confirm(&mut app, KeyCode::Enter);
        if cursor == 3 {
            assert!(app.world().resource::<SaveFiles>().active());
        } else {
            assert_ne!(
                app.world().resource::<MenuState>().screen,
                MenuScreen::Command
            );
        }
        assert_eq!(
            sounds(&mut app),
            [AudioRequest::play_sound("decision", &[100, 100])]
        );
    }
}

#[test]
fn original_cannot_act_states_block_the_skill_member_prompt() {
    let screen = MenuScreen::MemberSelect {
        action: MemberAction::Skill,
        cursor: 0,
    };
    for state in [1, 3, 4, 7, 8, 9, 10] {
        let mut app = with_sounds(1, screen);
        let mut vitals = app.world_mut().resource_mut::<Vitals>();
        if state == 1 {
            vitals.set(1, 0, 0);
        } else {
            vitals.set_states(1, vec![state]);
        }
        confirm(&mut app, KeyCode::Enter);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen,
            "state {state}"
        );
        assert_eq!(
            sounds(&mut app),
            [AudioRequest::play_sound("buzzer", &[100, 100])]
        );
    }
}

#[test]
fn other_states_allow_skills_and_knockout_does_not_block_equipment() {
    for state in [2, 5, 6] {
        let mut app = with_sounds(
            1,
            MenuScreen::MemberSelect {
                action: MemberAction::Skill,
                cursor: 0,
            },
        );
        app.world_mut()
            .resource_mut::<Vitals>()
            .set_states(1, vec![state]);
        confirm(&mut app, KeyCode::Enter);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::SkillList {
                member: 0,
                cursor: 0
            }
        );
        assert_eq!(
            sounds(&mut app),
            [AudioRequest::play_sound("decision", &[100, 100])]
        );
    }
    let mut app = with_sounds(
        2,
        MenuScreen::MemberSelect {
            action: MemberAction::Equip,
            cursor: 0,
        },
    );
    app.world_mut().resource_mut::<Vitals>().set(1, 0, 0);
    confirm(&mut app, KeyCode::Enter);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::Equip {
            member: 0,
            slot: 0,
            picking: None
        }
    );
}
