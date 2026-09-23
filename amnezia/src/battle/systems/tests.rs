//! Integration tests for the battle plugin's systems (start/reward/outcome/audio).

use super::*;
use crate::audio::BgmTrack;
use amnezia_data::{ActorDef, MonsterDef, MusicDef, SkillDef, SoundDef, TroopDef, TroopMemberDef};
use bevy::input::ButtonInput;

mod drops;
mod rewards;
mod timer;
mod transitions;

#[test]
fn renamed_hero_keeps_the_runtime_name_when_a_battle_starts() {
    let mut app = logic_app();
    app.insert_resource(crate::text::HeroName("Áron".into()));
    app.world_mut().write_message(BattleRequest {
        troop_id: DEBUG_TROOP,
        ..default()
    });
    app.update();
    finish_transition(&mut app);
    assert_eq!(app.world().resource::<Battle>().members[0].name, "Áron");
    assert_eq!(
        app.world().resource::<GameData>().actor(1).unwrap().name,
        "Ron"
    );
}

#[test]
fn draining_an_animation_acknowledges_even_an_effect_shorter_than_a_render_update() {
    use bevy::ecs::system::RunSystemOnce;
    let mut world = World::new();
    let mut battle = Battle::default();
    battle.begin_anim_hold();
    battle.push_anim(1, vec![(0.0, 0.0)]);
    world.insert_resource(battle);
    world.init_resource::<Messages<PlayAnimation>>();
    world.run_system_once(drain_pending_anims).unwrap();
    let mut battle = world.resource_mut::<Battle>();
    assert!(battle.pending_anims.is_empty());
    assert!(
        !battle.tick_anim_hold(false),
        "no render-count-dependent grace delay"
    );
}

#[derive(Resource, Default)]
struct AudioLog(Vec<AudioRequest>);

fn finish_transition(app: &mut App) {
    for _ in 0..160 {
        if !app.world().resource::<crate::battle::BattleFlow>().busy() {
            return;
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<crate::timing::GameFrames>()
            .frame += 1;
        app.update();
    }
    panic!("battle transition did not finish");
}

fn confirm_outcome(app: &mut App) {
    for _ in 0..4000 {
        if app.world().resource::<crate::battle::BattleFlow>().busy() {
            return;
        }
        let ready = crate::dialogue::testing::ready(app.world());
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        if ready {
            keys.press(KeyCode::Enter);
        }
        app.world_mut()
            .resource_mut::<crate::timing::GameFrames>()
            .frame += 1;
        app.update();
    }
    panic!("battle outcome did not finish its dialogue");
}

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

#[test]
fn poison_survives_battles_but_temporary_sleep_does_not() {
    let mut app = logic_app();
    app.world_mut().resource_mut::<BattleData>().states = crate::conditions::definitions().to_vec();
    app.world_mut()
        .resource_mut::<Vitals>()
        .set_states(1, vec![2]);
    app.world_mut().write_message(BattleRequest {
        troop_id: DEBUG_TROOP,
        ..default()
    });
    app.update();
    finish_transition(&mut app);
    assert_eq!(
        app.world().resource::<Battle>().members[0].states,
        vec![(2, 0)]
    );
    {
        let mut battle = app.world_mut().resource_mut::<Battle>();
        battle.members[0].states.push((7, 0));
        battle.finish(BattleOutcome::Escape);
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(app.world().resource::<Vitals>().states(1), vec![2]);
}

/// A headless app wired with just the battle logic systems (no UI, which
/// needs an asset server) plus the resources they read.
fn logic_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    crate::dialogue::testing::register_playback(&mut app);
    app.add_plugins(crate::transitions::TransitionPlugin);
    app.add_message::<BattleRequest>();
    app.add_message::<AudioRequest>();
    app.insert_resource(GameData {
        actors: vec![ActorDef {
            character_name: String::new(),
            character_index: 0,
            rename_skill: false,
            skill_name: String::new(),
            critical_hit: false,
            critical_hit_chance: 30,
            state_ranks: Vec::new(),
            attribute_ranks: Vec::new(),
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
            battler_hue: 0,
            drop_id: 0,
            drop_prob: 100,
            critical_hit: false,
            critical_hit_chance: 30,
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
            pages: Vec::new(),
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
    app.insert_resource(crate::text::HeroName("Ron".into()));
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<AudioLog>();
    crate::battle::flow::register(&mut app);
    app.add_systems(
        PostUpdate,
        |mut messages: MessageReader<AudioRequest>, mut log: ResMut<AudioLog>| {
            log.0.extend(messages.read().cloned());
        },
    );
    app.add_systems(
        Update,
        (
            debug_trigger,
            start_on_request,
            apply_victory_rewards.before(outcome_input),
            outcome_input.after(crate::dialogue::MessageUpdate),
        ),
    );
    app
}

#[test]
fn f6_starts_a_battle_and_confirming_a_win_publishes_the_contract_out() {
    let mut app = logic_app();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F6);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    finish_transition(&mut app);
    assert!(
        app.world().resource::<BattleActive>().0,
        "battle should be running"
    );
    {
        let battle = app.world().resource::<Battle>();
        assert!(battle.phase == Phase::Encounter);
        assert_eq!(battle.enemies.len(), 1);
        assert_eq!(battle.members.len(), 1);
    }
    app.world_mut()
        .resource_mut::<Battle>()
        .finish(BattleOutcome::Victory);
    confirm_outcome(&mut app);
    finish_transition(&mut app);
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
fn memorize_and_restore_round_trips_the_map_bgm() {
    let mut memory = MapBgm::default();
    assert_eq!(memory.restore(), AudioRequest::StopBgm);
    memory.memorize(Some(BgmTrack {
        name: "Field".into(),
        volume: 0.8,
        speed: 1.0,
        fade_in: 0.0,
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
    memory.memorize(None);
    assert_eq!(memory.restore(), AudioRequest::StopBgm);
}

#[test]
fn battle_start_plays_battle_music_and_stores_the_prior_bgm() {
    let mut app = logic_app();
    app.insert_resource(CurrentBgm::with_track("Field", 0.7, 1.0));
    // Request the fight directly (avoids the debug-key intra-frame ordering).
    app.world_mut().write_message(BattleRequest {
        troop_id: DEBUG_TROOP,
        background: "Town".into(),
        first_strike: true,
        ..default()
    });
    app.update();
    finish_transition(&mut app);
    let battle = app.world().resource::<Battle>();
    assert_eq!(battle.background, "Town");
    assert!(!battle.allow_escape);
    assert!(battle.first_strike);
    assert_eq!(
        app.world().resource::<MapBgm>().0,
        Some(BgmTrack {
            name: "Field".into(),
            volume: 0.7,
            speed: 1.0,
            fade_in: 0.0,
        })
    );
    let played = &app.world().resource::<AudioLog>().0;
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
