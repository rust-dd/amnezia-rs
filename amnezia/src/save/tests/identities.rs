use super::*;
use crate::gamedata::GameDataPlugin;

mod duplicates;
mod originals;
mod protection;

fn load_case(
    tag: &str,
    version: u32,
    change: impl FnOnce(&mut SaveGame),
) -> (App, PathBuf, String) {
    let path = temp_slot(tag);
    let mut game = ron::from_str::<SaveGame>(
        "(map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:0)",
    )
    .unwrap();
    game.format_version = version;
    change(&mut game);
    write_save(&path, &game).unwrap();
    let original = std::fs::read_to_string(&path).unwrap();
    let mut app = save_app(path.clone());
    app.add_plugins(GameDataPlugin)
        .init_resource::<RunningEvent>();
    app.world_mut().resource_mut::<Switches>().set(888, true);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    (app, path, original)
}

fn unchanged_file(path: PathBuf, original: String) {
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn legacy_actor_item_skill_and_condition_references_are_repaired() {
    for version in 0..=14 {
        let (app, path, original) =
            load_case(&format!("legacy_database_ids_{version}"), version, |game| {
                game.items = vec![(181, 2), (999, 7)];
                game.progression = vec![(999, 0)];
                game.vitals = vec![(999, (5, 5))];
                game.learned_skills = vec![(1, vec![1, 999, 1]), (999, vec![1])];
                game.conditions = vec![(1, vec![2, 999, 2]), (999, vec![2])];
                game.equipment = vec![(1, [181, 999, 64, 83, 0]), (999, [1, 0, 0, 0, 0])];
            });
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert_eq!(app.world().resource::<Inventory>().count(999), 0);
        assert_eq!(app.world().resource::<Inventory>().count(181), 2);
        assert!(app.world().resource::<Progression>().entries().is_empty());
        assert!(app.world().resource::<Vitals>().entries().is_empty());
        assert_eq!(
            app.world().resource::<Progression>().skill_entries(),
            [(1, vec![1])]
        );
        assert_eq!(
            app.world().resource::<Vitals>().condition_entries(),
            [(1, vec![2])]
        );
        assert_eq!(
            app.world().resource::<Equipment>().entries(),
            [(1, [0, 0, 64, 83, 0])]
        );
        unchanged_file(path, original);
    }
}

#[test]
fn current_invalid_identities_are_rejected_before_live_state_changes() {
    for case in 0..19 {
        let (app, path, original) = load_case(
            &format!("current_identity_error_{case}"),
            SAVE_FORMAT_VERSION,
            |game| match case {
                0 => game.party = vec![1, 1],
                1 => game.party = vec![999],
                2 => game.party = vec![1, 2, 3, 4, 5],
                3 => game.items = vec![(999, 1)],
                4 => game.items = vec![(181, 1), (181, 2)],
                5 => game.progression = vec![(999, 0)],
                6 => game.learned_skills = vec![(1, vec![999])],
                7 => game.conditions = vec![(1, vec![999])],
                8 => game.equipment = vec![(1, [181, 0, 0, 0, 0])],
                9 => game.vitals = vec![(999, (1, 1))],
                10 => game.party.clear(),
                11 => game.variables = vec![(0, 42)],
                12 => game.switches = vec![(1, true), (1, false)],
                13 => game.variables = vec![(1, 42), (1, 0)],
                14 => game.progression = vec![(1, 0), (1, 30)],
                15 => game.vitals = vec![(1, (1, 1)), (1, (2, 2))],
                16 => game.learned_skills = vec![(1, vec![1, 1])],
                17 => game.conditions = vec![(1, vec![2, 2])],
                _ => game.equipment = vec![(1, [0; 5]), (1, [1, 0, 0, 0, 0])],
            },
        );
        assert_eq!(
            app.world().resource::<LoadOutcome>().0,
            Some(false),
            "case {case}"
        );
        assert!(app.world().resource::<Switches>().get(888));
        assert!(app.world().resource::<PendingTeleport>().0.is_none());
        unchanged_file(path, original);
    }
}

#[test]
fn legacy_rosters_keep_the_first_four_unique_existing_actors_in_order() {
    for version in 0..=14 {
        let (app, path, original) =
            load_case(&format!("legacy_roster_ids_{version}"), version, |game| {
                game.party = vec![2, 2, 999, 1, 0, 3, 4, 5]
            });
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert_eq!(app.world().resource::<Party>().snapshot(), [2, 1, 3, 4]);
        unchanged_file(path, original);
    }
}
