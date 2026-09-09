use super::MapEvents;
use crate::state::{Inventory, Party, Switches, Variables, active_page};

pub(super) fn event_blocks_at(
    events: &MapEvents,
    state: (&Switches, &Variables, &Party, &Inventory),
    (self_id, layer): (u32, u32),
    (x, y): (i32, i32),
) -> bool {
    events.events.iter().any(|event| {
        event.id != self_id
            && (event.x as i32, event.y as i32) == (x, y)
            && active_page(event, state.0, state.1, state.2, state.3)
                .is_some_and(|page| page.layer == layer)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{asset_root, load_ron};
    use amnezia_data::Map;

    #[test]
    fn airship_cast_can_cross_ron_without_blocking_its_graphic_commands() {
        let map = load_ron::<Map>(&format!("{}/maps/map_0094.ron", asset_root()));
        let mut ron = map
            .events
            .iter()
            .find(|event| event.name == "Ron")
            .unwrap()
            .clone();
        ron.x = 9;
        ron.y = 9;
        let events = MapEvents { events: vec![ron] };
        let switches = Switches::default();
        let variables = Variables::default();
        let party = Party::default();
        let inventory = Inventory::default();
        let state = (&switches, &variables, &party, &inventory);
        for event in map
            .events
            .iter()
            .filter(|event| (3..=6).contains(&event.id))
        {
            assert!(
                !event_blocks_at(&events, state, (event.id, event.pages[0].layer), (9, 9)),
                "{} must be able to cross Ron's tile",
                event.name
            );
        }
        assert!(event_blocks_at(&events, state, (0, 1), (9, 9)));
        assert!(!event_blocks_at(&events, state, (2, 1), (9, 9)));
    }

    #[test]
    fn characters_block_their_own_layer_including_above_and_below_the_hero() {
        let map = load_ron::<Map>(&format!("{}/maps/map_0094.ron", asset_root()));
        let mut events = MapEvents {
            events: vec![map.events[1].clone()],
        };
        let switches = Switches::default();
        let variables = Variables::default();
        let party = Party::default();
        let inventory = Inventory::default();
        let state = (&switches, &variables, &party, &inventory);
        for other in 0..=2 {
            events.events[0].pages[0].layer = other;
            for layer in 0..=2 {
                assert_eq!(
                    event_blocks_at(&events, state, (0, layer), (9, 8)),
                    layer == other
                );
            }
        }
        events.events[0].pages[0].condition.flags = 1;
        events.events[0].pages[0].condition.switch_a = 238;
        assert!(!event_blocks_at(&events, state, (0, 2), (9, 8)));
    }
}
