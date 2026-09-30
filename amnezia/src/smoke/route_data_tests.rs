use crate::assets::{asset_root, load_ron};
use crate::state::{Inventory, Party, Switches, Variables, active_page_index};
use amnezia_data::{Chipset, EventCommand, Map};
use std::collections::BTreeSet;

fn map(id: u32) -> Map {
    load_ron(&format!("{}/maps/map_{id:04}.ron", asset_root()))
}

fn page(map: &Map, event: u32, page: usize) -> &[EventCommand] {
    &map.events
        .iter()
        .find(|entry| entry.id == event)
        .unwrap()
        .pages[page - 1]
        .commands
}

fn flight(start: (i32, i32), command: &EventCommand) -> BTreeSet<(i32, i32)> {
    assert_eq!(command.code, 11330);
    let mut tile = start;
    let mut tiles = BTreeSet::from([tile]);
    for code in &command.params[4..] {
        let delta = match code {
            0 => (0, -1),
            1 => (1, 0),
            2 => (0, 1),
            3 => (-1, 0),
            12 | 13 | 14 | 15 | 23 | 28 | 29 => continue,
            _ => panic!("unexpected original flight command {code}"),
        };
        tile.0 += delta.0;
        tile.1 += delta.1;
        tiles.insert(tile);
    }
    tiles
}

fn timed_flight(start: (i32, i32), command: &EventCommand, frames: u32) -> BTreeSet<(i32, i32)> {
    let mut remaining = frames;
    let mut speed = 5u32;
    let mut tile = start;
    let mut tiles = BTreeSet::from([tile]);
    for code in &command.params[4..] {
        let delta = match code {
            0 => (0, -1),
            1 => (1, 0),
            2 => (0, 1),
            3 => (-1, 0),
            28 => {
                speed = (speed + 1).min(6);
                continue;
            }
            29 => {
                speed = (speed - 1).max(1);
                continue;
            }
            _ => panic!("unexpected timed flight command {code}"),
        };
        let duration = 1 << (7 - speed);
        if remaining < duration {
            break;
        }
        remaining -= duration;
        tile.0 += delta.0;
        tile.1 += delta.1;
        tiles.insert(tile);
    }
    tiles
}

#[test]
fn original_murder_discovery_stops_the_two_minute_clock_and_promotes_both_choices() {
    let deck = map(126);
    let timers = page(&deck, 34, 1)
        .iter()
        .filter(|command| command.code == 10230)
        .map(|command| command.params.clone())
        .collect::<Vec<_>>();
    assert_eq!(timers, [vec![0, 0, 120, 0, 0], vec![1, 0, 300, 0, 0]]);
    let room = map(129);
    let chest = room.events.iter().find(|event| event.id == 5).unwrap();
    let mut switches = Switches::default();
    switches.set(323, true);
    switches.set(324, true);
    let party = Party::default();
    let variables = Variables::default();
    let inventory = Inventory::default();
    assert_eq!(
        active_page_index(chest, &switches, &variables, &party, &inventory),
        Some(1)
    );
    assert_eq!(chest.pages[1].commands[0].params, [2, 0, 0, 0, 0]);
    assert!(
        chest.pages[1]
            .commands
            .iter()
            .any(|command| command.code == 10210 && command.params == [0, 326, 326, 0])
    );
    switches.set(326, true);
    assert_eq!(
        active_page_index(chest, &switches, &variables, &party, &inventory),
        Some(2)
    );
    let deltas = chest.pages[2]
        .commands
        .iter()
        .filter(|command| command.code == 10220)
        .map(|command| command.params.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        deltas,
        [vec![0, 1, 1, 2, 0, 1, 0], vec![0, 1, 1, 1, 0, 1, 0]]
    );
}

#[test]
fn both_original_return_flights_match_the_independent_tile_oracle() {
    let dock = map(132);
    let departure = page(&dock, 38, 1);
    let route_index = departure
        .iter()
        .position(|command| command.code == 11330 && command.params.len() > 20)
        .unwrap();
    let escape = &departure[route_index];
    assert_eq!(departure[route_index + 1].code, 11410);
    assert_eq!(departure[route_index + 1].params, [40]);
    assert_eq!(departure[route_index + 2].code, 11010);
    let deck = map(126);
    let home = page(&deck, 27, 6)
        .iter()
        .find(|command| {
            command.code == 11330
                && command.params.first() == Some(&10004)
                && command.params.len() > 20
        })
        .unwrap();
    let mut tiles = timed_flight((28, 100), escape, 40 * 6);
    assert_eq!(
        flight((28, 100), escape)
            .difference(&tiles)
            .copied()
            .collect::<BTreeSet<_>>(),
        (45..=49).map(|x| (x, 99)).collect()
    );
    tiles.extend(flight((54, 99), home));
    assert_eq!(tiles, super::return_trip::expected_flight());
    assert!(tiles.contains(&(44, 99)) && tiles.contains(&(92, 113)));
}

#[test]
fn original_hospital_consumes_the_music_handoff_and_leaves_walkable_floor() {
    let hospital = map(154);
    let commands = page(&hospital, 53, 1);
    assert_eq!(commands.len(), 2);
    assert_eq!(
        (commands[0].code, commands[0].string.as_str()),
        (11510, "Hospital")
    );
    assert_eq!(commands[1].params, [0, 307, 307, 1]);
    let chipset = load_ron::<Vec<Chipset>>(&format!("{}/chipsets.ron", asset_root()))
        .into_iter()
        .find(|chip| chip.id == hospital.chipset_id)
        .unwrap();
    for (from, to) in [((14, 12), (14, 13)), ((14, 13), (14, 12))] {
        let index = (from.1 * hospital.width as i32 + from.0) as usize;
        assert!(crate::tiles::passable(
            hospital.lower[index],
            hospital.upper[index],
            &chipset.passages_down,
            &chipset.passages_up,
            crate::tiles::passable_mask(from.0, from.1, to.0, to.1)
        ));
    }
}

#[test]
fn the_original_pilot_changes_when_stark_joins_the_current_party() {
    let deck = map(126);
    let pilot = deck.events.iter().find(|event| event.id == 27).unwrap();
    let mut switches = Switches::default();
    switches.set(405, true);
    let variables = Variables::default();
    let inventory = Inventory::default();
    let mut party = Party::default();
    for (page, expected) in [(8, 2), (9, 4)] {
        if page == 9 {
            party.add(8);
        }
        assert_eq!(
            active_page_index(pilot, &switches, &variables, &party, &inventory),
            Some(page)
        );
        assert_eq!(pilot.pages[page].graphic_index, expected);
        let options = pilot.pages[page]
            .commands
            .iter()
            .filter(|command| command.code == 10140)
            .map(|command| (command.string.as_str(), command.params.as_slice()))
            .collect::<Vec<_>>();
        assert_eq!(
            options,
            [
                ("Igen!/Igen, de ne a kastélyhoz.../Még nem...", &[3][..]),
                ("Igen.../Hát, most, hogy mondod...", &[2][..])
            ]
        );
    }
}

#[test]
fn original_briefing_unlocks_departure_and_keeps_dianos_optional() {
    let castle = map(160);
    let entrances = castle
        .events
        .iter()
        .flat_map(|event| &event.pages)
        .flat_map(|page| &page.commands)
        .filter(|command| command.code == 10810 && command.params.first() == Some(&126))
        .collect::<Vec<_>>();
    assert_eq!(entrances.len(), 4);
    assert!(
        entrances
            .iter()
            .all(|command| command.params == [126, 9, 11])
    );
    let deck = map(126);
    let pilot = deck.events.iter().find(|event| event.id == 27).unwrap();
    let mut switches = Switches::default();
    switches.set(402, true);
    assert_eq!(
        active_page_index(
            pilot,
            &switches,
            &Variables::default(),
            &Party::default(),
            &Inventory::default()
        ),
        Some(7)
    );
    let briefing = &pilot.pages[7].commands;
    for (id, value) in [(48, 13), (49, 93), (50, 60), (53, 4)] {
        assert!(briefing.iter().any(|command| {
            command.code == 10220 && command.params == [0, id, id, 0, 0, value, 0]
        }));
    }
    assert_eq!(briefing.last().unwrap().params, [0, 405, 405, 0]);
    assert!(
        briefing
            .iter()
            .any(|command| { command.code == 12010 && command.params == [0, 323, 0, 0, 0, 0] })
    );
    let island = map(170);
    let commands = page(&island, 8, 1);
    let jump = commands
        .iter()
        .position(|command| {
            command.code == 11330
                && command.params.first() == Some(&7)
                && command.params.contains(&24)
        })
        .unwrap();
    assert_eq!(commands[jump - 1].code, 12010);
    assert_eq!(commands[jump - 1].params, [0, 301, 0, 0, 0, 0]);
    assert_eq!(commands[jump].indent, commands[jump - 1].indent + 1);
}

#[test]
fn the_original_sky_flight_and_eight_island_jumps_reach_the_staged_cast() {
    let sky = map(171);
    let command = page(&sky, 1, 1)
        .iter()
        .find(|command| command.code == 11330)
        .unwrap();
    assert_eq!(
        flight((10, 54), command),
        (7..=54).map(|y| (10, y)).collect()
    );
    let island = map(170);
    let jumps = page(&island, 8, 1)
        .iter()
        .filter(|command| command.code == 11330 && command.params.contains(&24))
        .collect::<Vec<_>>();
    assert_eq!(jumps.len(), 8);
    for command in jumps {
        let id = command.params[0] as u32;
        let actor = island.events.iter().find(|event| event.id == id).unwrap();
        let start = command.params.iter().position(|code| *code == 24).unwrap();
        let end = command.params.iter().position(|code| *code == 25).unwrap();
        assert!(command.params[start + 1..end].iter().all(|code| *code == 0));
        let tile = (actor.x as i32, actor.y as i32 - (end - start - 1) as i32);
        let expected = match id {
            1 => (16, 16),
            2 => (13, 17),
            3 => (12, 17),
            4 => (18, 18),
            5 => (19, 18),
            6 => (17, 17),
            7 => (21, 17),
            8 => (15, 16),
            _ => unreachable!(),
        };
        assert_eq!(tile, expected);
    }
}

#[test]
fn the_original_tower_entrance_and_return_preserve_the_saved_flight_location() {
    let world = map(13);
    let commands = page(&world, 58, 1);
    for params in [vec![1, 63, 0, 54, 0, 0], vec![1, 64, 0, 35, 0, 0]] {
        assert!(
            commands
                .iter()
                .any(|command| command.code == 12010 && command.params == params)
        );
    }
    assert!(
        commands
            .iter()
            .any(|command| command.code == 10810 && command.params == [249, 12, 2])
    );
    let tower = map(249);
    assert!(
        page(&tower, 38, 1)
            .iter()
            .any(|command| command.code == 10830 && command.params == [48, 49, 50])
    );
    let entry = page(&tower, 40, 1);
    assert!(
        entry
            .iter()
            .any(|command| command.code == 12010 && command.params == [0, 532, 1, 0, 0, 0])
    );
    assert!(
        entry
            .iter()
            .any(|command| command.code == 10210 && command.params == [0, 532, 532, 0])
    );
}
