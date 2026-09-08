//! Integration tests for the battle plugin's systems (start/reward/outcome/audio).

use super::*;
use crate::audio::BgmTrack;
use amnezia_data::{MonsterDef, MusicDef, SkillDef, SoundDef, TroopDef, TroopMemberDef};
use bevy::input::ButtonInput;

/// A System audio def with a battle track, start SE, and the per-hit effects
/// set, so the battle-audio systems have names to play.
fn test_system() -> SystemDef {
    let music = |name: &str| MusicDef {
        name: name.into(),
        volume: 90,
        tempo: 100,
        ..default()
    };
    let sound = |name: &str| SoundDef {
        name: name.into(),
        volume: 90,
        tempo: 100,
        ..default()
    };
    SystemDef {
        battle_music: music("Battle"),
        battle_end_music: music("Victory"),
        gameover_music: music("Gameover"),
        battle_se: sound("Start"),
        enemy_damaged_se: sound("Damage"),
        actor_damaged_se: sound("Bite"),
        dodge_se: sound("Evasion"),
        enemy_defeated_se: sound("Die"),
        escape_se: sound("Escape"),
        ..default()
    }
}

/// A headless app wired with just the battle logic systems (no UI, which
/// needs an asset server) plus the resources they read.
fn logic_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<BattleRequest>();
    app.add_message::<AudioRequest>();
    app.insert_resource(GameData {
        actors: vec![ActorDef {
            id: 1,
            name: "Ron".into(),
            title: "Zsoldos".into(),
            level: 2,
            max_level: 50,
            hp: 63,
            sp: 37,
            curves: Default::default(),
            learnings: Vec::new(),
            exp_base: 30,
            exp_inflation: 30,
            exp_correction: 0,
            weapon: 0,
            shield: 0,
            armor: 0,
            helmet: 0,
            accessory: 0,
            two_weapons: false,
            fix_equipment: false,
            unarmed_animation: 0,
            face_name: String::new(),
            face_index: 0,
        }],
        items: vec![],
        skills: vec![],
    });
    app.insert_resource(BattleData {
        monsters: vec![MonsterDef {
            id: 1,
            name: "Rabló".into(),
            battler: "Cannibal".into(),
            max_hp: 30,
            max_sp: 0,
            attack: 20,
            defense: 8,
            spirit: 0,
            agility: 8,
            exp: 10,
            gold: 30,
            attribute_ranks: vec![],
            state_ranks: vec![],
            actions: vec![],
        }],
        troops: vec![TroopDef {
            id: DEBUG_TROOP,
            name: "Rablo".into(),
            members: vec![TroopMemberDef {
                enemy_id: 1,
                x: 100,
                y: 100,
            }],
        }],
        attributes: vec![],
        states: vec![],
        system: test_system(),
    });
    app.init_resource::<Party>();
    app.init_resource::<Inventory>();
    app.init_resource::<Vitals>();
    app.init_resource::<Progression>();
    app.init_resource::<Equipment>();
    app.init_resource::<Battle>();
    app.init_resource::<BattleActive>();
    app.init_resource::<BattleResult>();
    app.init_resource::<MapBgm>();
    app.init_resource::<CurrentBgm>();
    app.init_resource::<Terms>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.add_systems(
        Update,
        (
            debug_trigger,
            start_on_request,
            apply_victory_rewards.before(outcome_input),
            outcome_input,
        ),
    );
    app
}

#[test]
fn f6_starts_a_battle_and_confirming_a_win_publishes_the_contract_out() {
    let mut app = logic_app();
    // F6 -> debug_trigger emits BattleRequest -> start_on_request builds it.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F6);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    assert!(
        app.world().resource::<BattleActive>().0,
        "battle should be running"
    );
    {
        let battle = app.world().resource::<Battle>();
        assert!(battle.phase == Phase::PartyCommand);
        assert_eq!(battle.enemies.len(), 1);
        assert_eq!(battle.members.len(), 1);
    }
    // Force a victory, then confirm at the outcome screen.
    app.world_mut()
        .resource_mut::<Battle>()
        .finish(BattleOutcome::Victory);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(
        app.world().resource::<BattleResult>().0,
        Some(BattleOutcome::Victory)
    );
    assert!(
        !app.world().resource::<BattleActive>().0,
        "battle should have ended"
    );
    assert_eq!(
        app.world().resource::<Inventory>().gold(),
        30,
        "gold reward paid out"
    );
    assert!(app.world().resource::<Battle>().phase == Phase::Inactive);
}

#[test]
fn a_threshold_victory_levels_up_before_the_outcome_and_pays_exactly_once() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Ron at level 1: reaching level 2 needs exp_for_level(2) = exp_base = 30.
    let ron = ActorDef {
        id: 1,
        name: "Ron".into(),
        title: String::new(),
        level: 1,
        max_level: 50,
        hp: 40,
        sp: 10,
        curves: Default::default(),
        // Ron learns skill 1 at level 2, so crossing into level 2 learns it.
        learnings: vec![amnezia_data::Learning {
            level: 2,
            skill_id: 1,
        }],
        exp_base: 30,
        exp_inflation: 30,
        exp_correction: 0,
        weapon: 0,
        shield: 0,
        armor: 0,
        helmet: 0,
        accessory: 0,
        two_weapons: false,
        fix_equipment: false,
        unarmed_animation: 0,
        face_name: String::new(),
        face_index: 0,
    };
    app.insert_resource(GameData {
        actors: vec![ron.clone()],
        items: vec![],
        skills: vec![SkillDef {
            id: 1,
            name: "Tűzcsapás".into(),
            description: String::new(),
            sp_cost: 0,
            power: 10,
            hit: 100,
            skill_type: 0,
            scope: 0,
            animation_id: 0,
            physical_rate: 0,
            magical_rate: 0,
            variance: 0,
            affect_hp: true,
            affect_sp: false,
            absorb: false,
            attributes: vec![],
            affected_states: vec![],
        }],
    });
    app.init_resource::<Inventory>();
    app.init_resource::<Vitals>();
    app.init_resource::<Progression>();
    app.init_resource::<BattleResult>();
    app.init_resource::<BattleActive>();
    app.init_resource::<MapBgm>();
    app.add_message::<AudioRequest>();
    app.init_resource::<ButtonInput<KeyCode>>();
    // A foe worth exactly 30 exp — enough to lift Ron from level 1 to 2 — and
    // 30 gold.
    let monsters = vec![MonsterDef {
        id: 1,
        name: "Rabló".into(),
        battler: String::new(),
        max_hp: 30,
        max_sp: 0,
        attack: 20,
        defense: 8,
        spirit: 0,
        agility: 8,
        exp: 30,
        gold: 30,
        attribute_ranks: vec![],
        state_ranks: vec![],
        actions: vec![],
    }];
    let troop = TroopDef {
        id: 1,
        name: "T".into(),
        members: vec![TroopMemberDef {
            enemy_id: 1,
            x: 100,
            y: 100,
        }],
    };
    let battle = Battle::build(
        &troop,
        &monsters,
        &[&ron],
        &[[0, 0, 0, 0, 0]],
        &[],
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        1,
    );
    app.insert_resource(battle);
    app.add_systems(
        Update,
        (apply_victory_rewards.before(outcome_input), outcome_input),
    );
    // Win the fight, then enter the outcome without confirming: the reward must
    // pay and the level rise now, before the outcome screen.
    app.world_mut()
        .resource_mut::<Battle>()
        .finish(BattleOutcome::Victory);
    app.update();
    assert_eq!(
        app.world().resource::<Progression>().level(&ron),
        2,
        "the level rises on entering the outcome, not next fight"
    );
    assert_eq!(
        app.world().resource::<Inventory>().gold(),
        30,
        "gold is paid once, at victory time"
    );
    assert!(
        app.world()
            .resource::<Battle>()
            .log
            .iter()
            .any(|l| l.contains("szintet")),
        "a level-up line is staged into the battle log"
    );
    assert!(
        app.world()
            .resource::<Battle>()
            .log
            .iter()
            .any(|l| l.contains("megtanulta") && l.contains("Tűzcsapás")),
        "crossing level 2 learns and logs the level-2 skill"
    );
    let total_once = app.world().resource::<Progression>().total(&ron);
    // Confirm at the outcome screen: gold and exp must not be applied a second
    // time.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(
        app.world().resource::<Inventory>().gold(),
        30,
        "gold is not double-paid on confirm"
    );
    assert_eq!(
        app.world().resource::<Progression>().total(&ron),
        total_once,
        "experience is not double-applied on confirm"
    );
}

#[test]
fn memorize_and_restore_round_trips_the_map_bgm() {
    let mut memory = MapBgm::default();
    // A silent map restores to a stop, not a phantom track.
    assert_eq!(memory.restore(), AudioRequest::StopBgm);
    memory.memorize(Some(BgmTrack {
        name: "Field".into(),
        volume: 0.8,
        speed: 1.0,
    }));
    assert_eq!(
        memory.restore(),
        AudioRequest::Bgm {
            name: "Field".into(),
            volume: 0.8,
            speed: 1.0,
            fade_in: 0.0,
        }
    );
    // Memorizing "nothing playing" restores to a stop.
    memory.memorize(None);
    assert_eq!(memory.restore(), AudioRequest::StopBgm);
}

#[test]
fn battle_start_plays_battle_music_and_stores_the_prior_bgm() {
    let mut app = logic_app();
    // A map track is playing when the fight starts.
    app.insert_resource(CurrentBgm::with_track("Field", 0.7, 1.0));
    // Request the fight directly (avoids the debug-key intra-frame ordering).
    app.world_mut().write_message(BattleRequest {
        troop_id: DEBUG_TROOP,
        background: "Town".into(),
        first_strike: true,
        ..default()
    });
    app.update();
    let battle = app.world().resource::<Battle>();
    assert_eq!(battle.background, "Town");
    assert!(!battle.allow_escape);
    assert!(battle.first_strike);
    // The pre-battle map BGM is remembered for the teardown restore.
    assert_eq!(
        app.world().resource::<MapBgm>().0,
        Some(BgmTrack {
            name: "Field".into(),
            volume: 0.7,
            speed: 1.0
        })
    );
    // The battle BGM and the battle-start SE were requested on build.
    let messages = app.world().resource::<Messages<AudioRequest>>();
    let mut cursor = messages.get_cursor();
    let played: Vec<AudioRequest> = cursor.read(messages).cloned().collect();
    assert!(
        played
            .iter()
            .any(|r| matches!(r, AudioRequest::Bgm { name, .. } if name.as_str() == "Battle")),
        "battle BGM should start on battle build: {played:?}"
    );
    assert!(
        played
            .iter()
            .any(|r| matches!(r, AudioRequest::Sound { name, .. } if name.as_str() == "Start")),
        "battle-start SE should sound"
    );
}
