use super::*;
use amnezia_data::{ActorDef, ItemDef, StateDef};

#[test]
fn campaign_states_and_actors_retain_their_distinct_resistance_tables() {
    let states = load_ron::<Vec<StateDef>>(&format!("{}/states.ron", asset_root()));
    assert_eq!(states.len(), 10);
    assert_eq!(states[0].rates, [80, 60, 40, 20, 0]);
    assert_eq!(states[1].rates, [90, 70, 50, 30, 0]);
    assert_eq!(states[8].rates, [100, 85, 70, 40, 0]);
    assert_eq!(states[2].reduce_hit_ratio, 20);
    assert_eq!(states[4].reduce_hit_ratio, 50);
    assert_eq!(states[4].affect_stats, [false, true, false, false]);
    assert_eq!(states[4].affect_type, 0);
    assert!(!states[3].restrict_skill && states[3].restrict_magic);
    assert_eq!(states[3].restrict_magic_level, 1);
    assert_eq!(
        (
            states[9].sp_change_type,
            states[9].sp_change_max,
            states[9].sp_change_val
        ),
        (0, 2, 1)
    );
    let actors = load_ron::<Vec<ActorDef>>(&format!("{}/actors.ron", asset_root()));
    assert_eq!(
        actors
            .iter()
            .filter(|actor| !actor.state_ranks.is_empty())
            .count(),
        6
    );
    assert_eq!(actors[0].state_ranks, [2, 2, 3, 2, 3]);
    assert_eq!(actors[4].state_ranks, [2, 1, 1, 4, 1, 3, 1, 1]);
    assert_eq!(actors[9].state_ranks, [4; 10]);
    let items = load_ron::<Vec<ItemDef>>(&format!("{}/items.ron", asset_root()));
    for (id, probability) in [(13, 50), (48, 75), (49, 100), (146, 100)] {
        assert_eq!(
            items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .state_chance,
            probability
        );
    }
}
