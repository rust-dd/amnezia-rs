use super::*;
use crate::world::EventSprite;
use crate::world::collision::{CollisionBodies, MapCollision};
use bevy::ecs::system::SystemParam;

#[derive(SystemParam)]
pub(super) struct Obstacles<'w, 's> {
    events: Res<'w, MapEvents>,
    variables: Res<'w, Variables>,
    party: Res<'w, Party>,
    inventory: Res<'w, Inventory>,
    characters:
        Query<'w, 's, (&'static EventSprite, Option<&'static RouteStepper>), Without<Player>>,
}

impl Obstacles<'_, '_> {
    pub(super) fn bodies(
        &self,
        vehicles: &Vehicles,
        map_id: u32,
        hero_through: bool,
    ) -> CollisionBodies {
        let mut bodies = CollisionBodies::from_events(self.characters.iter());
        bodies.hero_through = hero_through;
        bodies.include_vehicles(Some(vehicles), map_id);
        bodies
    }

    pub(super) fn collision<'a>(
        &'a self,
        data: &'a MapData,
        switches: &'a Switches,
        bodies: &'a CollisionBodies,
    ) -> MapCollision<'a> {
        MapCollision::new(
            data,
            &self.events,
            (switches, &self.variables, &self.party, &self.inventory),
            bodies,
        )
    }

    pub(super) fn blocks_disembarking(&self, tile: (i32, i32), switches: &Switches) -> bool {
        self.events.events.iter().any(|event| {
            (event.x as i32, event.y as i32) == tile
                && active_page(
                    event,
                    switches,
                    &self.variables,
                    &self.party,
                    &self.inventory,
                )
                .is_some_and(|page| page.layer == 1)
        })
    }
}
