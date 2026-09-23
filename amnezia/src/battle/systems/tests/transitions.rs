use super::*;
use crate::transitions::{Defaults, Kind, Transition};

#[test]
fn unhandled_defeat_hands_directly_to_game_over_without_revealing_or_replaying_the_map() {
    for unhandled in [true, false] {
        let mut app = logic_app();
        app.insert_resource(Defaults([0, 0, 19, 19, 19, 16]));
        app.insert_resource(CurrentBgm::with_track("Field", 0.7, 1.0));
        app.world_mut().write_message(BattleRequest {
            troop_id: DEBUG_TROOP,
            defeat_ends_game: unhandled,
            ..default()
        });
        frame(&mut app, 0);
        frame(&mut app, 21);
        frame(&mut app, 22);
        app.world_mut()
            .resource_mut::<Battle>()
            .finish(BattleOutcome::Defeat);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        frame(&mut app, 23);
        assert!(app.world().resource::<crate::dialogue::Dialogue>().active);
        app.world_mut()
            .resource_mut::<crate::dialogue::Dialogue>()
            .close();
        frame(&mut app, 23);
        frame(&mut app, 24);
        frame(&mut app, 25);
        assert_eq!(
            app.world().resource::<crate::gameover::GameOverActive>().0,
            unhandled
        );
        assert_eq!(app.world().resource::<BattleActive>().0, !unhandled);
        assert_eq!(app.world().resource::<Transition>().erased(), unhandled);
        assert_eq!(
            app.world().resource::<BattleResult>().0,
            unhandled.then_some(BattleOutcome::Defeat)
        );
        let audio = &app.world().resource::<AudioLog>().0;
        assert_eq!(
            audio.iter().any(
                |request| matches!(request, AudioRequest::Bgm { name, .. } if name == "Field")
            ),
            !unhandled
        );
        if !unhandled {
            frame(&mut app, 66);
            assert_eq!(
                app.world().resource::<BattleResult>().0,
                Some(BattleOutcome::Defeat)
            );
        }
    }
}

fn frame(app: &mut App, value: u32) {
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = value;
    app.update();
}

#[test]
fn original_battle_entry_and_exit_hold_scene_input_and_result_until_each_transition_finishes() {
    let mut app = logic_app();
    app.insert_resource(Defaults([0, 0, 16, 17, 17, 16]));
    app.add_systems(
        Update,
        crate::battle::input::command_input
            .run_if(crate::battle::flow::playing)
            .after(crate::battle::flow::drive),
    );
    app.world_mut().write_message(BattleRequest {
        troop_id: DEBUG_TROOP,
        ..default()
    });
    frame(&mut app, 0);
    assert!(app.world().resource::<BattleActive>().0);
    assert_eq!(app.world().resource::<Battle>().phase, Phase::Inactive);
    frame(&mut app, 60);
    assert_eq!(app.world().resource::<Battle>().phase, Phase::Inactive);
    frame(&mut app, 61);
    assert_eq!(app.world().resource::<Battle>().phase, Phase::Encounter);
    assert!(app.world().resource::<crate::battle::BattleFlow>().busy());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    frame(&mut app, 101);
    assert_eq!(app.world().resource::<Battle>().phase, Phase::Encounter);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    assert!(app.world().resource::<crate::battle::BattleFlow>().busy());
    frame(&mut app, 102);
    assert!(!app.world().resource::<crate::battle::BattleFlow>().busy());
    app.world_mut()
        .resource_mut::<Battle>()
        .finish(BattleOutcome::Abort);
    frame(&mut app, 103);
    frame(&mut app, 104);
    frame(&mut app, 144);
    assert_eq!(app.world().resource::<Battle>().phase, Phase::Outcome);
    assert_eq!(app.world().resource::<BattleResult>().0, None);
    frame(&mut app, 145);
    assert_eq!(app.world().resource::<Battle>().phase, Phase::Inactive);
    assert!(app.world().resource::<BattleActive>().0);
    frame(&mut app, 185);
    assert_eq!(app.world().resource::<BattleResult>().0, None);
    frame(&mut app, 186);
    assert_eq!(
        app.world().resource::<BattleResult>().0,
        Some(BattleOutcome::Abort)
    );
    assert!(!app.world().resource::<BattleActive>().0);
    assert!(!app.world().resource::<Transition>().erased());
}

#[test]
fn troop_pages_do_not_start_during_the_battle_show_transition() {
    use crate::state::{Switches, Variables};
    use amnezia_data::{EventCommand, TroopPageConditionDef, TroopPageDef};
    let mut app = logic_app();
    app.insert_resource(Defaults([0, 0, 16, 17, 17, 16]));
    app.init_resource::<Switches>()
        .init_resource::<Variables>()
        .init_resource::<crate::dialogue::Dialogue>()
        .add_systems(
            Update,
            crate::battle::events::drive.run_if(crate::battle::flow::playing),
        );
    app.world_mut().resource_mut::<BattleData>().troops[0].pages = vec![TroopPageDef {
        condition: TroopPageConditionDef {
            flags: 8,
            ..default()
        },
        commands: vec![EventCommand {
            code: 10210,
            indent: 0,
            string: String::new(),
            params: vec![0, 9001, 9001, 0],
        }],
    }];
    app.world_mut().write_message(BattleRequest {
        troop_id: DEBUG_TROOP,
        ..default()
    });
    frame(&mut app, 0);
    frame(&mut app, 61);
    frame(&mut app, 101);
    assert!(!app.world().resource::<Switches>().get(9001));
    frame(&mut app, 102);
    assert!(!app.world().resource::<Switches>().get(9001));
    for _ in 0..80 {
        let mut battle = app.world_mut().resource_mut::<Battle>();
        if battle.phase == Phase::Encounter {
            crate::battle::message::encounter::advance(&mut battle, default());
        }
    }
    frame(&mut app, 103);
    assert!(app.world().resource::<Switches>().get(9001));
}

#[test]
fn already_erased_encounter_waits_forty_frames_and_returns_to_an_event_erased_map() {
    let mut app = logic_app();
    app.insert_resource(Defaults([0, 0, 16, 19, 19, 16]));
    app.world_mut()
        .resource_mut::<Transition>()
        .start(Kind::Cut, true, 0, IVec2::ZERO);
    frame(&mut app, 1);
    app.world_mut().resource_mut::<Transition>().event_erased = true;
    app.world_mut().write_message(BattleRequest {
        troop_id: DEBUG_TROOP,
        ..default()
    });
    frame(&mut app, 2);
    frame(&mut app, 41);
    assert_eq!(app.world().resource::<Battle>().phase, Phase::Inactive);
    frame(&mut app, 42);
    assert_eq!(app.world().resource::<Battle>().phase, Phase::Encounter);
    frame(&mut app, 43);
    app.world_mut()
        .resource_mut::<Battle>()
        .finish(BattleOutcome::Abort);
    frame(&mut app, 44);
    frame(&mut app, 45);
    frame(&mut app, 46);
    frame(&mut app, 47);
    assert!(!app.world().resource::<BattleActive>().0);
    assert!(app.world().resource::<Transition>().erased());
    assert!(app.world().resource::<Transition>().event_erased);
}
