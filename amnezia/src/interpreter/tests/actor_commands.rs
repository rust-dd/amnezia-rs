use super::*;
use crate::assets::{asset_root, load_ron};
use crate::equipment::Equipment;
use crate::gamedata::GameDataPlugin;

fn game_app() -> App {
    let mut app = interp_app();
    app.add_plugins(GameDataPlugin);
    app
}

#[test]
fn player_visibility_zero_hides_and_one_shows() {
    let mut app = game_app();
    run(&mut app, vec![cmd(11310, 0, vec![0])]);
    assert!(app.world().resource::<HeroHidden>().0);
    run(&mut app, vec![cmd(11310, 0, vec![1])]);
    assert!(!app.world().resource::<HeroHidden>().0);
}

#[test]
fn scripted_defeat_revival_restores_one_hp_without_healing_other_actors() {
    let mut app = game_app();
    app.world_mut().resource_mut::<Vitals>().set(1, 0, 7);
    app.world_mut().resource_mut::<Vitals>().set(2, 0, 9);
    run(&mut app, vec![cmd(10480, 0, vec![1, 1, 1, 1])]);
    let vitals = app.world().resource::<Vitals>();
    assert_eq!(vitals.get_stored(1), Some((1, 7)));
    assert_eq!(vitals.get_stored(2), Some((0, 9)));
}

#[test]
fn original_poison_removal_cures_the_party() {
    let mut app = game_app();
    app.world_mut()
        .resource_mut::<Vitals>()
        .set_states(1, vec![2]);
    run(&mut app, vec![cmd(10480, 0, vec![0, 0, 1, 2])]);
    assert!(app.world().resource::<Vitals>().states(1).is_empty());
}

fn run(app: &mut App, commands: Vec<EventCommand>) {
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, commands);
    app.update();
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn original_story_rewards_teach_skills_to_the_selected_actor() {
    let mut app = game_app();
    let map = load_ron::<amnezia_data::Map>(&format!("{}/maps/map_0045.ron", asset_root()));
    let rewards = map
        .events
        .iter()
        .flat_map(|e| &e.pages)
        .flat_map(|p| &p.commands)
        .filter(|c| c.code == 10440)
        .cloned()
        .collect::<Vec<_>>();
    assert!(!rewards.is_empty());
    run(&mut app, rewards);
    let data = app.world().resource::<GameData>();
    let progression = app.world().resource::<Progression>();
    assert!(
        progression
            .known_skill_ids(data.actor(2).unwrap())
            .contains(&10)
    );
    assert!(
        !progression
            .known_skill_ids(data.actor(1).unwrap())
            .contains(&10)
    );
}

#[test]
fn learned_and_forgotten_skills_survive_experience_and_restore() {
    let mut app = game_app();
    app.world_mut().resource_mut::<Variables>().set(20, 3);
    run(&mut app, vec![cmd(10440, 0, vec![1, 1, 0, 1, 20])]);
    let actor = app.world().resource::<GameData>().actor(1).unwrap().clone();
    let mut progression = app.world_mut().resource_mut::<Progression>();
    assert!(progression.known_skill_ids(&actor).contains(&3));
    progression.change_skill(&actor, 3, false);
    progression.add(&actor, 1);
    assert!(!progression.known_skill_ids(&actor).contains(&3));
    progression.change_skill(&actor, 68, true);
    let exp = progression.entries();
    let skills = progression.skill_entries();
    let mut restored = Progression::default();
    restored.load(exp);
    restored.load_skills(skills);
    assert_eq!(
        restored.known_skill_ids(&actor),
        progression.known_skill_ids(&actor)
    );
    restored.set_level(&actor, 1);
    assert!(restored.known_skill_ids(&actor).contains(&68));
}

#[test]
fn original_weapon_exchange_changes_the_live_loadout() {
    let mut app = game_app();
    let map = load_ron::<amnezia_data::Map>(&format!("{}/maps/map_0116.ron", asset_root()));
    let changes = map
        .events
        .iter()
        .flat_map(|e| &e.pages)
        .flat_map(|p| &p.commands)
        .filter(|c| c.code == 10450)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(changes.len(), 2);
    let actor = app.world().resource::<GameData>().actor(1).unwrap().clone();
    let old_weapon = actor.weapon;
    run(&mut app, changes);
    assert_eq!(app.world().resource::<Equipment>().slots(&actor)[0], 3);
    if old_weapon != 0 {
        assert_eq!(app.world().resource::<Inventory>().count(old_weapon), 1);
    }
}

#[test]
fn full_heal_respects_actor_and_party_targets() {
    let mut app = game_app();
    app.world_mut().resource_mut::<Vitals>().set(1, 0, 0);
    app.world_mut().resource_mut::<Vitals>().set(2, 1, 0);
    run(&mut app, vec![cmd(10490, 0, vec![1, 1])]);
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), None);
    assert_eq!(app.world().resource::<Vitals>().get_stored(2), Some((1, 0)));
    run(&mut app, vec![cmd(10490, 0, vec![0, 0])]);
    assert_eq!(app.world().resource::<Vitals>().get_stored(2), Some((1, 0)));
    app.world_mut().resource_mut::<Party>().add(2);
    run(&mut app, vec![cmd(10490, 0, vec![0, 0])]);
    assert_eq!(app.world().resource::<Vitals>().get_stored(2), None);
}
