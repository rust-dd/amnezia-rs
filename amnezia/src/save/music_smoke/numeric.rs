use super::*;

pub(super) fn legacy_values(game: &mut SaveGame) {
    game.gold = i32::MAX;
    game.items = vec![(181, u32::MAX)];
    game.variables.extend([(9905, i32::MIN), (9906, i32::MAX)]);
    game.progression = vec![(2, u32::MAX)];
    game.vitals = vec![(1, (2301, 2302))];
    game.timer_remaining = -3.5;
    game.timer_running = false;
    game.timer_visible = false;
}

pub(super) fn verify(world: &World) {
    assert_eq!(world.resource::<Vitals>().get_stored(1), Some((63, 37)));
    let inventory = world.resource::<Inventory>();
    assert_eq!(inventory.gold(), 999_999);
    assert_eq!(inventory.count(181), 99);
    let variables = world.resource::<Variables>();
    assert_eq!(variables.get(9905), -999_999);
    assert_eq!(variables.get(9906), 999_999);
    let tiff = world
        .resource::<crate::gamedata::GameData>()
        .actor(2)
        .unwrap();
    assert_eq!(world.resource::<Progression>().total(tiff), 999_999);
    assert_eq!(world.resource::<GameClock>().remaining, 0.0);
    info!("saved numbers: legacy Ron HP/SP, inventory, variables, EXP and timer repaired");
}
