use super::*;
use crate::assets::{asset_root, load_ron};
use crate::battle::{BattleData, BattleOutcome};
use amnezia_data::{EventCommand, MonsterDef, SystemDef, TroopDef, TroopPageConditionDef};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[path = "tests/scheduling.rs"]
mod scheduling;

#[derive(Resource, Default)]
struct AudioLog(Vec<&'static str>);

fn record_audio(mut audio: MessageReader<AudioRequest>, mut log: ResMut<AudioLog>) {
    for request in audio.read() {
        log.0.push(match request {
            AudioRequest::StopBgm => "stop",
            AudioRequest::Sound { .. } => "sound",
            _ => "music",
        });
    }
}

fn app(troop_id: u32, members: &[u32]) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )))
        .add_message::<AudioRequest>()
        .init_resource::<Dialogue>()
        .init_resource::<Switches>()
        .init_resource::<Variables>()
        .init_resource::<Equipment>()
        .init_resource::<Progression>()
        .init_resource::<Vitals>()
        .init_resource::<AudioLog>()
        .add_systems(Update, (drive, record_audio).chain());
    let data = GameData {
        actors: load_ron(&format!("{}/actors.ron", asset_root())),
        items: load_ron(&format!("{}/items.ron", asset_root())),
        skills: load_ron(&format!("{}/skills.ron", asset_root())),
    };
    let troops = load_ron::<Vec<TroopDef>>(&format!("{}/troops.ron", asset_root()));
    let monsters = load_ron::<Vec<MonsterDef>>(&format!("{}/monsters.ron", asset_root()));
    let troop = troops.iter().find(|t| t.id == troop_id).unwrap();
    let mut party = Party::default();
    party.restore(members.to_vec());
    let actors = members
        .iter()
        .map(|id| data.actor(*id).unwrap())
        .collect::<Vec<_>>();
    let battle = Battle::build(
        troop,
        &monsters,
        &actors,
        &[],
        &data.items,
        &[],
        &[],
        &data.skills,
        &Vitals::default(),
        &Progression::default(),
        String::new(),
        1,
    );
    app.insert_resource(data)
        .insert_resource(party)
        .insert_resource(battle)
        .insert_resource(BattleData {
            monsters,
            troops,
            attributes: vec![],
            states: vec![],
            system: SystemDef::default(),
        });
    app
}

fn settle(app: &mut App) {
    for _ in 0..300 {
        app.world_mut().resource_mut::<Dialogue>().active = false;
        app.world_mut()
            .resource_mut::<Battle>()
            .advance_deaths(1.0 / 60.0);
        app.update();
        let battle = app.world().resource::<Battle>();
        if !battle.events.blocks_action() || battle.phase == Phase::Outcome {
            return;
        }
    }
    panic!("Troop event did not settle");
}

fn turn(app: &mut App, turn: u32) {
    {
        let mut battle = app.world_mut().resource_mut::<Battle>();
        assert_eq!(battle.events.turn + 1, turn);
        battle.events.next_turn();
    }
    settle(app);
}

#[test]
fn original_cyclone_tutorial_waits_animates_teaches_and_aborts() {
    let mut app = app(15, &[1]);
    app.update();
    assert!(!app.world().resource::<Dialogue>().active);
    app.update();
    assert!(app.world().resource::<Dialogue>().active);
    assert!(
        !app.world().resource::<Battle>().members[0]
            .known_skills
            .contains(&2)
    );
    settle(&mut app);
    let battle = app.world().resource::<Battle>();
    assert_eq!(battle.outcome, Some(BattleOutcome::Abort));
    assert_eq!(battle.events.turn, 0);
    assert!(battle.members[0].known_skills.contains(&2));
    assert_eq!(battle.pending_anims.len(), 1);
    assert_eq!(battle.pending_anims[0].anim_id, 4);
    assert_eq!(battle.pending_anims[0].targets.len(), 3);
    let actor = app.world().resource::<GameData>().actor(1).unwrap();
    assert!(
        app.world()
            .resource::<Progression>()
            .known_skill_ids(actor)
            .contains(&2)
    );
}

#[test]
fn original_alen_joins_on_turn_three_without_resetting_ron() {
    let mut app = app(16, &[1]);
    app.world_mut().resource_mut::<Battle>().members[0].hp = 17;
    app.world_mut().resource_mut::<Vitals>().set(4, 23, 5);
    settle(&mut app);
    assert!(app.world().resource::<Switches>().get(194));
    for n in 1..=2 {
        turn(&mut app, n);
    }
    assert_eq!(app.world().resource::<Battle>().members.len(), 1);
    turn(&mut app, 3);
    let battle = app.world().resource::<Battle>();
    assert_eq!(
        battle
            .members
            .iter()
            .map(|f| f.actor_id)
            .collect::<Vec<_>>(),
        [1, 4]
    );
    assert_eq!(battle.members[0].hp, 17);
    assert_eq!((battle.members[1].hp, battle.members[1].sp), (23, 5));
    assert!(battle.members[1].command.is_none());
    assert_eq!(app.world().resource::<Party>().snapshot(), [1, 4]);
    turn(&mut app, 4);
    assert_eq!(app.world().resource::<Battle>().members.len(), 2);
}

#[test]
fn original_lance_stops_music_and_aborts_on_turn_three() {
    let mut app = app(28, &[1, 2]);
    settle(&mut app);
    for n in 1..=2 {
        turn(&mut app, n);
    }
    assert!(app.world().resource::<Battle>().outcome.is_none());
    turn(&mut app, 3);
    assert_eq!(
        app.world().resource::<Battle>().outcome,
        Some(BattleOutcome::Abort)
    );
    assert!(app.world().resource::<AudioLog>().0.contains(&"stop"));
}

#[test]
fn original_daren_learns_tigers_roar_on_turn_three() {
    let mut app = app(38, &[3]);
    settle(&mut app);
    for n in 1..=3 {
        turn(&mut app, n);
    }
    assert!(
        app.world().resource::<Battle>().members[0]
            .known_skills
            .contains(&18)
    );
    assert!(app.world().resource::<Battle>().outcome.is_none());
}

#[test]
fn original_six_page_training_heals_before_each_lesson_and_aborts_at_six() {
    let mut app = app(50, &[1]);
    settle(&mut app);
    for n in 1..=6 {
        app.world_mut().resource_mut::<Battle>().enemies[0].hp = 2;
        turn(&mut app, n);
        let battle = app.world().resource::<Battle>();
        assert_eq!(
            battle.enemies[0].hp,
            if n == 1 { 2 } else { battle.enemies[0].max_hp }
        );
        assert_eq!(battle.outcome, (n == 6).then_some(BattleOutcome::Abort));
    }
}

#[test]
fn original_final_battle_pages_recheck_hp_after_actions_but_run_once_per_turn() {
    for (troop, first_switch) in [(31, 475), (51, 616)] {
        let mut app = app(troop, &[1, 2, 3, 4]);
        settle(&mut app);
        assert!(app.world().resource::<Switches>().get(first_switch));
        app.world_mut().resource_mut::<Battle>().enemies[0].hp = 1;
        turn(&mut app, 1);
        assert_eq!(app.world().resource::<Battle>().enemies[0].hp, 3000);
        app.world_mut()
            .resource_mut::<Battle>()
            .events
            .check_pages();
        settle(&mut app);
        assert_eq!(app.world().resource::<Battle>().enemies[0].hp, 3000);
        {
            let mut battle = app.world_mut().resource_mut::<Battle>();
            battle.enemies[1].hp = 0;
            battle.enemies[3].hp = 0;
            battle.events.check_pages();
        }
        settle(&mut app);
        assert!(app.world().resource::<Switches>().get(617));
        assert!(app.world().resource::<Switches>().get(545));
        app.world_mut().resource_mut::<Switches>().set(545, false);
        turn(&mut app, 2);
        assert!(!app.world().resource::<Switches>().get(545));
        turn(&mut app, 3);
        assert!(app.world().resource::<Switches>().get(545));
    }
}

fn command(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent: 0,
        string: String::new(),
        params,
    }
}

fn install(app: &mut App, conditions: Vec<TroopPageConditionDef>) {
    let pages = conditions
        .into_iter()
        .enumerate()
        .map(|(index, condition)| TroopPageDef {
            condition,
            commands: vec![command(
                10210,
                vec![0, 800 + index as i32, 800 + index as i32, 0],
            )],
        })
        .collect::<Vec<_>>();
    app.world_mut().resource_mut::<Battle>().events = BattleEvents::new(&pages);
}

#[test]
fn pages_require_a_trigger_and_all_enabled_conditions_in_database_order() {
    let mut app = app(2, &[1]);
    install(
        &mut app,
        vec![
            TroopPageConditionDef::default(),
            TroopPageConditionDef {
                flags: 1 | 2 | 4 | 8,
                switch_a_id: 801,
                switch_b_id: 802,
                variable_id: 12,
                variable_value: -7,
                ..default()
            },
            TroopPageConditionDef {
                flags: 8,
                ..default()
            },
        ],
    );
    app.world_mut().resource_mut::<Switches>().set(801, true);
    app.world_mut().resource_mut::<Variables>().set(12, -8);
    settle(&mut app);
    assert!(!app.world().resource::<Switches>().get(800));
    assert!(app.world().resource::<Switches>().get(802));
    assert!(!app.world().resource::<Battle>().events.executed[1]);
    app.world_mut().resource_mut::<Variables>().set(12, -7);
    app.world_mut()
        .resource_mut::<Battle>()
        .events
        .check_pages();
    settle(&mut app);
    assert!(app.world().resource::<Battle>().events.executed[1]);
}

#[test]
fn actor_hp_conditions_read_live_hp_and_persistent_absent_actors() {
    let mut app = app(2, &[1]);
    install(
        &mut app,
        vec![
            TroopPageConditionDef {
                flags: 64,
                actor_id: 1,
                actor_hp_min: 0,
                actor_hp_max: 0,
                ..default()
            },
            TroopPageConditionDef {
                flags: 64,
                actor_id: 2,
                actor_hp_min: 0,
                actor_hp_max: 0,
                ..default()
            },
        ],
    );
    app.world_mut().resource_mut::<Vitals>().set(1, 0, 0);
    app.world_mut().resource_mut::<Vitals>().set(2, 0, 0);
    settle(&mut app);
    assert!(!app.world().resource::<Switches>().get(800));
    assert!(app.world().resource::<Switches>().get(801));
}

#[test]
fn a_waiting_animation_blocks_learning_until_its_battle_slot_finishes() {
    let mut app = app(15, &[1]);
    app.init_resource::<ActiveAnimations>();
    app.world_mut().resource_mut::<ActiveAnimations>().total = 1;
    for _ in 0..100 {
        app.world_mut().resource_mut::<Dialogue>().active = false;
        app.update();
        if app.world().resource::<Battle>().events.wait_animation {
            break;
        }
    }
    assert!(app.world().resource::<Battle>().events.wait_animation);
    app.world_mut().resource_mut::<ActiveAnimations>().battle = 1;
    for _ in 0..120 {
        app.update();
    }
    assert!(
        !app.world().resource::<Battle>().members[0]
            .known_skills
            .contains(&2)
    );
    app.world_mut().resource_mut::<ActiveAnimations>().battle = 0;
    settle(&mut app);
    assert!(
        app.world().resource::<Battle>().members[0]
            .known_skills
            .contains(&2)
    );
}

#[test]
fn next_turn_starts_after_command_selection_not_when_reopening_the_menu() {
    let mut app = app(16, &[1]);
    settle(&mut app);
    app.world_mut().resource_mut::<Battle>().begin_resolve();
    assert_eq!(app.world().resource::<Battle>().events.turn, 1);
    settle(&mut app);
    app.world_mut().resource_mut::<Battle>().new_round();
    assert_eq!(app.world().resource::<Battle>().events.turn, 1);
    app.world_mut().resource_mut::<Battle>().begin_resolve();
    assert_eq!(app.world().resource::<Battle>().events.turn, 2);
}

#[test]
fn death_ends_battle_before_scheduling_a_new_hp_conditioned_page() {
    let mut app = app(2, &[1]);
    install(
        &mut app,
        vec![TroopPageConditionDef {
            flags: 32,
            enemy_hp_min: 0,
            enemy_hp_max: 0,
            ..default()
        }],
    );
    for enemy in &mut app.world_mut().resource_mut::<Battle>().enemies {
        enemy.hp = 0;
    }
    settle(&mut app);
    assert_eq!(
        app.world().resource::<Battle>().outcome,
        Some(BattleOutcome::Victory)
    );
    assert!(!app.world().resource::<Switches>().get(800));
}
