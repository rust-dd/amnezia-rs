use super::*;
use crate::state::{Inventory, Party, Switches, Variables, active_page_index};

#[test]
fn the_original_donation_page_changes_after_the_first_relationship_point() {
    let map = load_ron::<Map>(&format!("{}/maps/map_0025.ron", asset_root()));
    let donation = map.events.iter().find(|event| event.id == 26).unwrap();
    let switches = Switches::default();
    let mut variables = Variables::default();
    let party = Party::default();
    let inventory = Inventory::default();
    assert_eq!(
        active_page_index(donation, &switches, &variables, &party, &inventory),
        Some(0),
    );
    variables.set(1, 1);
    assert_eq!(
        active_page_index(donation, &switches, &variables, &party, &inventory),
        Some(1),
    );
}

#[test]
fn returning_to_draco_alone_after_the_briefing_does_not_repeat_the_dream() {
    let map = load_ron::<Map>(&format!("{}/maps/map_0125.ron", asset_root()));
    let dream = map.events.iter().find(|event| event.id == 23).unwrap();
    let mut switches = Switches::default();
    let variables = Variables::default();
    let party = Party::default();
    let inventory = Inventory::default();
    switches.set(316, true);
    let page = crate::state::active_page(dream, &switches, &variables, &party, &inventory).unwrap();
    assert_eq!(page.trigger, 3);
    switches.set(405, true);
    let page = crate::state::active_page(dream, &switches, &variables, &party, &inventory).unwrap();
    assert_eq!(page.trigger, 0);
    assert!(page.commands.iter().all(|command| command.code == 0));
}
