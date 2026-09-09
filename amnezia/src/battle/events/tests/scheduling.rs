use super::*;
use crate::battle::model::{Action, Command, Source};

#[test]
fn the_live_action_driver_runs_join_events_before_the_third_rounds_first_action() {
    let mut app = app(16, &[1]);
    app.init_resource::<crate::state::Inventory>()
        .init_resource::<ActiveAnimations>()
        .add_systems(Update, crate::battle::systems::resolve_tick.after(drive));
    settle(&mut app);
    for round in 1..=3 {
        {
            let mut battle = app.world_mut().resource_mut::<Battle>();
            assert_eq!(battle.phase, Phase::PartyCommand);
            battle.members[0].command = Some(Command::Defend);
            battle.enemies[0].actions.clear();
            battle.begin_resolve();
            battle.queue = vec![Action {
                source: Source::Party(0),
                kind: Command::Defend,
                agility: 1,
            }];
        }
        for _ in 0..180 {
            let before = app.world().resource::<Battle>().queue_at;
            app.world_mut().resource_mut::<Dialogue>().active = false;
            app.update();
            let battle = app.world().resource::<Battle>();
            if round == 3 && before == 0 && battle.queue_at > 0 {
                assert_eq!(battle.members.len(), 2);
                assert_eq!(battle.members[1].actor_id, 4);
            }
            if battle.phase == Phase::PartyCommand {
                break;
            }
        }
        assert_eq!(app.world().resource::<Battle>().phase, Phase::PartyCommand);
    }
    assert_eq!(app.world().resource::<Battle>().members.len(), 2);
}

#[test]
fn closing_a_troop_dialogue_does_not_also_confirm_a_battle_command() {
    let mut app = app(16, &[1]);
    app.init_resource::<crate::state::Inventory>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, crate::battle::input::command_input.after(drive));
    app.update();
    assert!(app.world().resource::<Dialogue>().active);
    app.world_mut().resource_mut::<Dialogue>().active = false;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(app.world().resource::<Battle>().phase, Phase::PartyCommand);
    *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = ButtonInput::default();
    app.update();
    assert!(!app.world().resource::<Battle>().events.blocks_action());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(app.world().resource::<Battle>().phase, Phase::Command);
}

#[test]
fn false_nested_branches_skip_to_their_own_else_and_success_skips_the_else_body() {
    for enabled in [true, false] {
        let mut app = app(2, &[1]);
        let mut commands = vec![
            command(13310, vec![0, 12, 0, 0, 0]),
            command(10210, vec![0, 800, 800, 0]),
            command(13310, vec![0, 13, 0, 0, 0]),
            command(10210, vec![0, 801, 801, 0]),
            command(23311, vec![]),
            command(23310, vec![]),
            command(10210, vec![0, 802, 802, 0]),
            command(23311, vec![]),
        ];
        for (c, indent) in commands.iter_mut().zip([0, 1, 1, 2, 1, 0, 1, 0]) {
            c.indent = indent;
        }
        app.world_mut().resource_mut::<Battle>().events = BattleEvents::new(&[TroopPageDef {
            condition: TroopPageConditionDef {
                flags: 8,
                ..default()
            },
            commands,
        }]);
        app.world_mut().resource_mut::<Switches>().set(12, enabled);
        settle(&mut app);
        let switches = app.world().resource::<Switches>();
        assert_eq!(switches.get(800), enabled);
        assert!(!switches.get(801));
        assert_eq!(switches.get(802), !enabled);
    }
}
