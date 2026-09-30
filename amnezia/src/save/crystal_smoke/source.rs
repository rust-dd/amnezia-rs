pub(super) const MAPS: [u32; 16] = [
    2, 18, 45, 74, 79, 91, 98, 111, 119, 125, 143, 145, 182, 202, 231, 260,
];

pub(super) fn crystal(id: u32) -> amnezia_data::Event {
    assert!(MAPS.contains(&id), "map {id} has no campaign save crystal");
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_{id:04}.ron",
        crate::assets::asset_root()
    ));
    let mut crystals = map.events.into_iter().filter(|event| {
        event
            .pages
            .iter()
            .any(|page| page.commands.iter().any(|c| c.code == 11910))
    });
    let event = crystals.next().unwrap();
    assert!(crystals.next().is_none());
    event
}

pub(super) fn checkpoint_switches(map: u32) -> Vec<(u32, bool)> {
    // These original completion flags place the checkpoint after its arrival cutscene.
    match map {
        45 => vec![(111, true), (123, true)],
        74 => vec![(186, true)],
        111 => vec![(291, true)],
        _ => Vec::new(),
    }
}

pub(super) fn approach(id: u32, crystal: &amnezia_data::Event) -> (i32, i32) {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_{id:04}.ron",
        crate::assets::asset_root()
    ));
    let chip = crate::assets::load_ron::<Vec<amnezia_data::Chipset>>(&format!(
        "{}/chipsets.ron",
        crate::assets::asset_root()
    ))
    .into_iter()
    .find(|chip| chip.id == map.chipset_id)
    .unwrap();
    let target = (crystal.x as i32, crystal.y as i32);
    let mut switches = crate::state::Switches::default();
    switches.load(checkpoint_switches(id));
    let variables = crate::state::Variables::default();
    let party = crate::state::Party::default();
    let inventory = crate::state::Inventory::default();
    [(0, 1), (-1, 0), (1, 0), (0, -1)]
        .into_iter()
        .map(|(dx, dy)| (target.0 + dx, target.1 + dy))
        .find(|&(x, y)| {
            if !(0..map.width as i32).contains(&x) || !(0..map.height as i32).contains(&y) {
                return false;
            }
            if map.events.iter().any(|event| {
                (event.x as i32, event.y as i32) == (x, y)
                    && crate::state::active_page(event, &switches, &variables, &party, &inventory)
                        .is_some_and(|page| page.layer == 1)
            }) {
                return false;
            }
            let from = (y * map.width as i32 + x) as usize;
            let to = (crystal.y * map.width + crystal.x) as usize;
            [
                crate::tiles::passable_mask(x, y, target.0, target.1),
                crate::tiles::passable_mask(target.0, target.1, x, y),
            ]
            .into_iter()
            .all(|mask| {
                [from, to].into_iter().all(|tile| {
                    crate::tiles::passable(
                        map.lower[tile],
                        map.upper[tile],
                        &chip.passages_down,
                        &chip.passages_up,
                        mask,
                    )
                })
            })
        })
        .unwrap_or_else(|| panic!("crystal on map {id} has no tile-passable approach"))
}

pub(super) fn key(from: (i32, i32), to: (i32, i32)) -> bevy::prelude::KeyCode {
    use bevy::prelude::KeyCode;
    match (to.0 - from.0, to.1 - from.1) {
        (0, -1) => KeyCode::ArrowUp,
        (1, 0) => KeyCode::ArrowRight,
        (0, 1) => KeyCode::ArrowDown,
        (-1, 0) => KeyCode::ArrowLeft,
        _ => panic!("crystal approach must be one tile away"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_final_crystal_approach_does_not_spawn_the_hero_inside_its_guard() {
        assert_eq!(approach(260, &crystal(260)), (15, 8));
    }

    #[test]
    fn every_campaign_crystal_has_the_original_walk_in_and_save_pages() {
        for id in MAPS {
            let event = crystal(id);
            let _ = key(approach(id, &event), (event.x as i32, event.y as i32));
            assert_eq!(event.pages.len(), 2, "map {id}");
            let walk = &event.pages[0];
            assert_eq!((walk.trigger, walk.layer, walk.animation_type), (1, 2, 5));
            assert!(walk.translucent);
            assert!(
                walk.commands
                    .iter()
                    .any(|c| c.code == 10210 && c.params == [0, 70, 70, 0])
            );
            let save = &event.pages[1];
            assert_eq!(
                (save.trigger, save.condition.flags, save.condition.switch_a),
                (0, 1, 70)
            );
            assert_eq!(
                save.commands.iter().map(|c| c.code).collect::<Vec<_>>(),
                [11550, 11910]
            );
        }
    }
}
