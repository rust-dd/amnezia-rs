use super::collision::{CollisionBodies, MapCollision};
use super::{EventSprite, MapData, MapEvents, MoveQueue, RouteStepper};
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::state::{Inventory, Party, Switches, Variables, active_page_index};
use amnezia_data::{Event, EventPage};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub(crate) struct EventTriggers<'w, 's> {
    pub(crate) data: Res<'w, MapData>,
    pub(crate) events: Res<'w, MapEvents>,
    switches: Res<'w, Switches>,
    variables: Res<'w, Variables>,
    party: Res<'w, Party>,
    inventory: Res<'w, Inventory>,
    sprites: Query<
        'w,
        's,
        (
            &'static mut EventSprite,
            Option<&'static mut RouteStepper>,
            Option<&'static MoveQueue>,
        ),
        Without<Player>,
    >,
}

impl EventTriggers<'_, '_> {
    pub(crate) fn page<'a>(&self, event: &'a Event) -> Option<(usize, &'a EventPage)> {
        let index = active_page_index(
            event,
            &self.switches,
            &self.variables,
            &self.party,
            &self.inventory,
        )?;
        Some((index, &event.pages[index]))
    }

    pub(crate) fn queue_at(
        &mut self,
        running: &mut RunningEvent,
        tile: (i32, i32),
        same_layer: bool,
        triggers: &[u32],
        hero: (i32, i32),
        decision: bool,
    ) -> bool {
        let events = self
            .events
            .events
            .iter()
            .filter_map(|event| {
                if (event.x as i32, event.y as i32) != tile {
                    return None;
                }
                let (index, page) = self.page(event)?;
                ((page.layer == 1) == same_layer
                    && triggers.contains(&page.trigger)
                    && !page.commands.is_empty())
                .then_some((event.id, index))
            })
            .collect::<Vec<_>>();
        let mut queued = false;
        for (id, index) in events {
            queued |= self.queue(running, id, index, hero, decision);
        }
        queued
    }

    pub(crate) fn queue(
        &mut self,
        running: &mut RunningEvent,
        id: u32,
        page: usize,
        hero: (i32, i32),
        decision: bool,
    ) -> bool {
        if !running.queue_event(self.data.map_id, id, page, decision) {
            return false;
        }
        if let Some((mut sprite, Some(mut route), _)) = self
            .sprites
            .iter_mut()
            .find(|(sprite, _, _)| sprite.id == id)
        {
            let delta = self.data.tile_delta((sprite.tile_x, sprite.tile_y), hero);
            let hero = (sprite.tile_x + delta.0, sprite.tile_y + delta.1);
            route.face_toward(&mut *sprite, hero);
        }
        true
    }

    pub(crate) fn stopped(&self, id: u32) -> bool {
        self.sprites
            .iter()
            .any(|(sprite, _, queue)| sprite.id == id && queue.is_none_or(|queue| !queue.busy()))
    }

    pub(crate) fn bodies(&self) -> CollisionBodies {
        CollisionBodies::from_events(
            self.sprites
                .iter()
                .map(|(sprite, route, _)| (sprite, route)),
        )
    }

    pub(crate) fn collision<'a>(&'a self, bodies: &'a CollisionBodies) -> MapCollision<'a> {
        MapCollision::new(
            &self.data,
            &self.events,
            (
                &self.switches,
                &self.variables,
                &self.party,
                &self.inventory,
            ),
            bodies,
        )
    }
}

pub(crate) fn finish_foreground(
    In(id): In<u32>,
    mut sprites: Query<(&mut EventSprite, &RouteStepper)>,
) {
    if let Some((mut sprite, route)) = sprites.iter_mut().find(|(sprite, _)| sprite.id == id) {
        route.update_facing(&mut *sprite);
    }
}
