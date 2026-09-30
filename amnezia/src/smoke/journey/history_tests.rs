use crate::assets::{asset_root, load_ron};
use crate::state::{Inventory, Party, Switches, Variables, active_page_index};
use amnezia_data::{Chipset, Map};
use bevy::prelude::*;

fn map(id: u32) -> Map {
    load_ron(&format!("{}/maps/map_{id:04}.ron", asset_root()))
}

#[test]
fn fortress_checkpoint_keeps_only_the_current_interior_alen() {
    let mut world = World::new();
    world.init_resource::<Switches>();
    world.init_resource::<Variables>();
    world.init_resource::<Party>();
    super::entry(&mut world);
    let interior = map(127);
    let party = Party::default();
    let inventory = Inventory::default();
    for murder in [false, true] {
        world.resource_mut::<Switches>().set(323, murder);
        let mut alens = Vec::new();
        let mut autoruns = Vec::new();
        for event in &interior.events {
            if let Some(index) = active_page_index(
                event,
                world.resource::<Switches>(),
                world.resource::<Variables>(),
                &party,
                &inventory,
            ) {
                let page = &event.pages[index];
                if page.graphic_name == "Chara4" && page.graphic_index == 0 {
                    alens.push((event.id, event.x, event.y));
                }
                if page.trigger == 3 {
                    autoruns.push(event.id);
                }
            }
        }
        assert_eq!(alens, [(36, 11, 5)], "murder branch: {murder}");
        assert_eq!(autoruns, [37], "only the fortress arrival may start");
    }
}

#[test]
fn checkpoint_history_preserves_the_original_completed_scene_switches() {
    let mut switches = Switches::default();
    switches.load(super::super::airship_history::switches(&[]));
    for (id, map_id, event_id, page) in [
        (312, 127, 9, 0),
        (313, 126, 27, 0),
        (315, 126, 27, 0),
        (316, 128, 2, 0),
        (321, 127, 30, 0),
        (300, 127, 30, 0),
        (322, 126, 27, 2),
        (324, 126, 34, 0),
    ] {
        let source = map(map_id);
        let event = source
            .events
            .iter()
            .find(|event| event.id == event_id)
            .unwrap();
        assert!(
            event.pages[page]
                .commands
                .iter()
                .any(|command| { command.code == 10210 && command.params == [0, id, id, 0] })
        );
        assert!(switches.get(id as u32));
    }
    for id in [314, 318, 319, 323, 326, 327, 329, 404] {
        assert!(!switches.get(id), "checkpoint must not invent branch {id}");
    }
}

#[test]
fn all_arrival_checkpoints_start_on_unoccupied_walkable_tiles() {
    let chipsets = load_ron::<Vec<Chipset>>(&format!("{}/chipsets.ron", asset_root()));
    let mut switches = Switches::default();
    switches.load(super::super::airship_history::switches(&[329]));
    let mut party = Party::default();
    party.add(2);
    for map_id in [125, 126, 127, 129] {
        let source = map(map_id);
        let chipset = chipsets
            .iter()
            .find(|chip| chip.id == source.chipset_id)
            .unwrap();
        let (x, y) = super::entry_tile(map_id as i32);
        let index = (y * source.width as i32 + x) as usize;
        assert!(
            crate::tiles::passable(
                source.lower[index],
                source.upper[index],
                &chipset.passages_down,
                &chipset.passages_up,
                crate::tiles::PASS_ALL,
            ),
            "map {map_id}: hero starts inside a wall"
        );
        for event in &source.events {
            if event.x != x as u32 || event.y != y as u32 {
                continue;
            }
            if let Some(index) = active_page_index(
                event,
                &switches,
                &Variables::default(),
                &party,
                &Inventory::default(),
            ) {
                assert_ne!(event.pages[index].layer, 1, "map {map_id}: occupied entry");
                assert_ne!(event.pages[index].trigger, 1, "map {map_id}: touch entry");
            }
        }
    }
}

#[test]
fn later_checkpoints_never_restore_the_first_meetings_cast() {
    let party = Party::default();
    let variables = Variables::default();
    let inventory = Inventory::default();
    for phase in [
        &[329][..],
        &[323][..],
        &[323, 327, 334, 627][..],
        &[327, 371, 402, 627][..],
        &[301, 327, 371, 402, 404, 405, 627][..],
    ] {
        let mut switches = Switches::default();
        switches.load(super::super::airship_history::switches(phase));
        for (map_id, event_id) in [(126, 31), (127, 9), (127, 12)] {
            let source = map(map_id);
            let event = source
                .events
                .iter()
                .find(|event| event.id == event_id)
                .unwrap();
            let index =
                active_page_index(event, &switches, &variables, &party, &inventory).unwrap();
            assert!(event.pages[index].graphic_name.is_empty());
            assert_eq!(event.pages[index].trigger, 0);
        }
    }
}
