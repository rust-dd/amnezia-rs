use super::{EventSprite, MapData, MapEvents, RouteStepper};
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::tiles::{ABOVE_HERO_BIT, PASS_ALL, passable_mask};
use amnezia_data::{Event, EventPage};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(crate) struct Mover {
    id: u32,
    layer: u32,
    tile: Option<u32>,
    through: bool,
}

impl Mover {
    pub(crate) fn event(event: &EventSprite, through: bool) -> Self {
        Self {
            id: event.id,
            layer: event.layer,
            tile: event.charset.is_empty().then_some(event.index),
            through,
        }
    }

    pub(crate) fn hero(through: bool) -> Self {
        Self {
            id: 0,
            layer: 1,
            tile: None,
            through,
        }
    }
}

#[derive(Default)]
pub(crate) struct CollisionBodies {
    events: HashMap<u32, Mover>,
    vehicles: Vec<((i32, i32), bool)>,
    pub(crate) hero_through: bool,
}

impl CollisionBodies {
    pub(crate) fn from_events<'a>(
        events: impl Iterator<Item = (&'a EventSprite, Option<&'a RouteStepper>)>,
    ) -> Self {
        Self {
            events: events
                .map(|(event, route)| {
                    (
                        event.id,
                        Mover::event(event, route.is_some_and(RouteStepper::through)),
                    )
                })
                .collect(),
            hero_through: false,
            vehicles: Vec::new(),
        }
    }

    pub(crate) fn update(&mut self, event: &EventSprite, route: &RouteStepper) {
        self.events
            .insert(event.id, Mover::event(event, route.through()));
    }

    pub(crate) fn include_vehicles(
        &mut self,
        vehicles: Option<&crate::vehicles::Vehicles>,
        map_id: u32,
    ) {
        if let Some(vehicles) = vehicles {
            self.hero_through |= vehicles.riding();
            self.vehicles = vehicles.collision_tiles(map_id).collect();
        }
    }

    fn event(&self, event: &Event, page: &EventPage) -> Mover {
        self.events.get(&event.id).copied().unwrap_or(Mover {
            id: event.id,
            layer: page.layer,
            tile: page.graphic_name.is_empty().then_some(page.graphic_index),
            through: false,
        })
    }
}

pub(crate) struct MapCollision<'a> {
    data: &'a MapData,
    events: &'a MapEvents,
    state: (&'a Switches, &'a Variables, &'a Party, &'a Inventory),
    bodies: &'a CollisionBodies,
}

impl<'a> MapCollision<'a> {
    pub(crate) fn new(
        data: &'a MapData,
        events: &'a MapEvents,
        state: (&'a Switches, &'a Variables, &'a Party, &'a Inventory),
        bodies: &'a CollisionBodies,
    ) -> Self {
        Self {
            data,
            events,
            state,
            bodies,
        }
    }

    fn page(&self, event: &'a Event) -> Option<&'a EventPage> {
        active_page(
            event,
            self.state.0,
            self.state.1,
            self.state.2,
            self.state.3,
        )
    }

    fn passage(&self, tile: u32) -> u8 {
        self.data
            .passages_up
            .get(tile as usize)
            .copied()
            .unwrap_or(0)
    }

    fn tile_passable(&self, position: (i32, i32), bit: u8, self_id: u32) -> bool {
        if !self.data.contains_tile(position.0, position.1) {
            return false;
        }
        let position = self.data.normalize_tile(position.0, position.1);
        let tile = self
            .events
            .events
            .iter()
            .filter_map(|event| {
                if event.id == self_id || (event.x as i32, event.y as i32) != position {
                    return None;
                }
                let page = self.page(event)?;
                let body = self.bodies.event(event, page);
                if body.layer != 0 || body.through {
                    return None;
                }
                body.tile.map(|tile| (event.id, tile))
            })
            .max_by_key(|(id, _)| *id)
            .map(|(_, tile)| tile);
        if let Some(tile) = tile.filter(|tile| *tile > 0) {
            let passage = self.passage(tile);
            if passage & ABOVE_HERO_BIT == 0 {
                return passage & bit != 0;
            }
        }
        self.data.passable_dir(position.0, position.1, bit)
    }

    fn event_blocks_at(&self, mover: Mover, position: (i32, i32), self_conflict: bool) -> bool {
        let forbidden = self
            .events
            .events
            .iter()
            .find(|event| event.id == mover.id)
            .and_then(|event| self.page(event))
            .is_some_and(|page| page.overlap_forbidden);
        self.events.events.iter().any(|event| {
            if event.id == mover.id || (event.x as i32, event.y as i32) != position {
                return false;
            }
            self.page(event).is_some_and(|page| {
                let other = self.bodies.event(event, page);
                !other.through
                    && (other.layer == mover.layer
                        || (self_conflict && other.layer == 1)
                        || (mover.id != 0 && (forbidden || page.overlap_forbidden)))
            })
        })
    }

    pub(crate) fn can_move(
        &self,
        from: (i32, i32),
        to: (i32, i32),
        mover: Mover,
        hero: Option<(i32, i32)>,
        jumping: bool,
    ) -> bool {
        if !jumping && from.0 != to.0 && from.1 != to.1 {
            let vertical = (from.0, to.1);
            let horizontal = (to.0, from.1);
            return (self.can_move(from, vertical, mover, hero, false)
                && self.can_move(vertical, to, mover, hero, false))
                || (self.can_move(from, horizontal, mover, hero, false)
                    && self.can_move(horizontal, to, mover, hero, false));
        }
        if !self.data.contains_tile(to.0, to.1) {
            return false;
        }
        if mover.through {
            return true;
        }
        let bit_from = passable_mask(from.0, from.1, to.0, to.1);
        let bit_to = if jumping {
            PASS_ALL
        } else {
            passable_mask(to.0, to.1, from.0, from.1)
        };
        if !jumping && !self.tile_passable(from, bit_from, mover.id) {
            return false;
        }
        let self_conflict = !jumping
            && mover.layer == 0
            && mover
                .tile
                .is_some_and(|tile| tile > 0 && self.passage(tile) & bit_from == 0);
        let destination = self.data.normalize_tile(to.0, to.1);
        if self.event_blocks_at(mover, destination, self_conflict)
            || self.bodies.vehicles.iter().any(|&(tile, airship)| {
                tile == destination
                    && (mover.layer == 1 || self_conflict)
                    && (mover.id != 0 || !airship)
            })
            || (mover.id != 0
                && !self.bodies.hero_through
                && hero == Some(destination)
                && (mover.layer == 1 || self_conflict))
        {
            return false;
        }
        self.tile_passable(destination, bit_to, mover.id)
    }
}

#[cfg(test)]
mod passage_tests;

#[cfg(test)]
mod route_tests;

#[cfg(test)]
fn event_blocks_at(
    events: &MapEvents,
    state: (&Switches, &Variables, &Party, &Inventory),
    (self_id, layer): (u32, u32),
    (x, y): (i32, i32),
) -> bool {
    MapCollision::new(
        &MapData::for_test(20, 20),
        events,
        state,
        &CollisionBodies::default(),
    )
    .event_blocks_at(
        Mover {
            id: self_id,
            layer,
            tile: None,
            through: false,
        },
        (x, y),
        false,
    )
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
