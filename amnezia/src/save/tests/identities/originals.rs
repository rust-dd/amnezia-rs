use super::*;
use crate::gamedata::GameData;

#[test]
fn original_actor_loadouts_and_database_ids_load_without_losing_items() {
    let mut definitions = App::new();
    definitions.add_plugins(GameDataPlugin);
    let data = definitions.world().resource::<GameData>();
    for actor in &data.actors {
        let slots = [
            actor.weapon,
            actor.shield,
            actor.armor,
            actor.helmet,
            actor.accessory,
        ];
        let skills = data.skills.iter().map(|skill| skill.id).collect::<Vec<_>>();
        let (app, path, original) = load_case(
            &format!("original_saved_ids_{}", actor.id),
            SAVE_FORMAT_VERSION,
            |game| {
                game.party = vec![actor.id];
                game.equipment = vec![(actor.id, slots)];
                game.items = data.items.iter().map(|item| (item.id, 1)).collect();
                game.learned_skills = vec![(actor.id, skills.clone())];
            },
        );
        assert_eq!(
            app.world().resource::<LoadOutcome>().0,
            Some(true),
            "actor {}",
            actor.id
        );
        assert_eq!(
            app.world().resource::<Equipment>().entries(),
            [(actor.id, slots)]
        );
        assert_eq!(
            app.world().resource::<Progression>().skill_entries(),
            [(actor.id, skills)]
        );
        for item in &data.items {
            assert_eq!(app.world().resource::<Inventory>().count(item.id), 1);
        }
        unchanged_file(path, original);
    }
}

#[test]
fn a_legacy_roster_with_no_surviving_member_is_not_replaced_with_an_invented_hero() {
    for version in 0..=SAVE_FORMAT_VERSION {
        let (app, path, original) =
            load_case(&format!("empty_saved_roster_{version}"), version, |game| {
                game.party = vec![0, 999];
            });
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(false));
        assert_eq!(app.world().resource::<Party>().snapshot(), [1]);
        assert!(app.world().resource::<Switches>().get(888));
        assert!(app.world().resource::<PendingTeleport>().0.is_none());
        unchanged_file(path, original);
    }
}
