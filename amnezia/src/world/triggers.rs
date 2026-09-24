use super::collision::{CollisionBodies, MapCollision};
use super::{MapData, MapEvents};
use crate::interpreter::RunningEvent;
use crate::state::{Inventory, Party, Switches, Variables, active_page_index};
use amnezia_data::{Event, EventPage};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(crate) struct EventTriggers<'w> {
    pub(crate) data: Res<'w, MapData>,
    pub(crate) events: Res<'w, MapEvents>,
    switches: Res<'w, Switches>,
    variables: Res<'w, Variables>,
    party: Res<'w, Party>,
    inventory: Res<'w, Inventory>,
}

impl EventTriggers<'_> {
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
        &self,
        running: &mut RunningEvent,
        tile: (i32, i32),
        same_layer: bool,
        triggers: &[u32],
    ) -> bool {
        let mut queued = false;
        for event in &self.events.events {
            if (event.x as i32, event.y as i32) != tile {
                continue;
            }
            if let Some((index, page)) = self.page(event)
                && (page.layer == 1) == same_layer
                && triggers.contains(&page.trigger)
                && !page.commands.is_empty()
            {
                queued |= running.queue_event(self.data.map_id, event.id, index);
            }
        }
        queued
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
