use super::*;

pub(super) fn legacy_values(game: &mut SaveGame) {
    game.party = vec![1, 1, 999, 2, 3, 4, 5];
    game.items.push((999, 3));
    game.equipment = vec![(1, [181, 999, 64, 83, 0])];
    game.learned_skills = vec![(1, vec![1, 999, 1])];
    game.conditions = vec![(1, vec![2, 999, 2])];
}

pub(super) fn verify(world: &World) {
    assert_eq!(world.resource::<Party>().snapshot(), [1, 2, 3, 4]);
    assert_eq!(world.resource::<Inventory>().count(999), 0);
    assert_eq!(
        world.resource::<Equipment>().entries(),
        [(1, [0, 0, 64, 83, 0])]
    );
    assert_eq!(
        world.resource::<Progression>().skill_entries(),
        [(1, vec![1])]
    );
    assert_eq!(
        world.resource::<Vitals>().condition_entries(),
        [(1, vec![2])]
    );
    info!(
        "saved identities: roster order, unique actors, item/gear/skill/state references verified"
    );
}
