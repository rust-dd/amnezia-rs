use super::*;

#[test]
fn legacy_duplicate_keys_keep_the_last_value_without_touching_the_file() {
    for version in 0..=14 {
        let (app, path, original) = load_case(
            &format!("legacy_duplicate_keys_{version}"),
            version,
            |game| {
                game.switches = vec![(0, true), (9999, true), (9999, false)];
                game.variables = vec![(0, 7), (9999, 42), (9999, 43)];
                game.items = vec![(181, 1), (181, 2)];
                game.progression = vec![(1, 0), (1, 30)];
                game.vitals = vec![(1, (1, 1)), (1, (2, 2))];
                game.learned_skills = vec![(1, vec![1]), (1, vec![2, 1, 2])];
                game.conditions = vec![(1, vec![3]), (1, vec![2])];
                game.equipment = vec![(1, [0; 5]), (1, [1, 0, 0, 0, 0])];
            },
        );
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert!(!app.world().resource::<Switches>().get(0));
        assert!(!app.world().resource::<Switches>().get(9999));
        assert_eq!(app.world().resource::<Variables>().get(0), 0);
        assert_eq!(app.world().resource::<Variables>().get(9999), 43);
        assert_eq!(app.world().resource::<Inventory>().count(181), 2);
        assert_eq!(app.world().resource::<Progression>().entries(), [(1, 30)]);
        assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 2)));
        assert_eq!(
            app.world().resource::<Progression>().skill_entries(),
            [(1, vec![1, 2])]
        );
        assert_eq!(
            app.world().resource::<Vitals>().condition_entries(),
            [(1, vec![2])]
        );
        assert_eq!(
            app.world().resource::<Equipment>().entries(),
            [(1, [1, 0, 0, 0, 0])]
        );
        unchanged_file(path, original);
    }
}
