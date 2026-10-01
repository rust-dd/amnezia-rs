use super::*;
use crate::assets::{asset_root, load_ron};
use amnezia_data::{Chipset, Map};

#[test]
fn the_captain_remains_reachable_after_leaving_the_draco_briefing_to_save() {
    let map = load_ron::<Map>(&format!("{}/maps/map_0126.ron", asset_root()));
    let chipsets = load_ron::<Vec<Chipset>>(&format!("{}/chipsets.ron", asset_root()));
    let chipset = chipsets
        .iter()
        .find(|chip| chip.id == map.chipset_id)
        .unwrap();
    let mut data = MapData::for_test(map.width as i32, map.height as i32);
    data.lower = map.lower;
    data.upper = map.upper;
    data.passages_down = chipset.passages_down.clone();
    data.passages_up = chipset.passages_up.clone();
    let events = MapEvents { events: map.events };
    let mut switches = Switches::default();
    switches.set(334, true);
    switches.set(402, true);
    let variables = Variables::default();
    let party = Party::default();
    let inventory = Inventory::default();
    let bodies = CollisionBodies::default();
    let collision = MapCollision::new(
        &data,
        &events,
        (&switches, &variables, &party, &inventory),
        &bodies,
    );
    assert!(!collision.can_move((9, 13), (9, 12), Mover::hero(false), None, false));

    switches.set(405, true);
    let collision = MapCollision::new(
        &data,
        &events,
        (&switches, &variables, &party, &inventory),
        &bodies,
    );
    for (from, to) in [((9, 13), (9, 12)), ((9, 12), (9, 11)), ((9, 11), (9, 10))] {
        assert!(collision.can_move(from, to, Mover::hero(false), None, false));
    }
    let captain = events.events.iter().find(|event| event.id == 27).unwrap();
    let page = active_page(captain, &switches, &variables, &party, &inventory).unwrap();
    assert!(page.commands.iter().any(|command| {
        command.code == 10140 && command.string.contains("Igen, de ne a kastélyhoz")
    }));
}
