use super::*;

impl MapCollision<'_> {
    pub(super) fn tile_passable(&self, position: (i32, i32), bit: u8, self_id: u32) -> bool {
        if !self.data.contains_tile(position.0, position.1) {
            return false;
        }
        let position = self.data.normalize_tile(position.0, position.1);
        let vehicle = vehicle_index(self_id);
        if let Some(index) = vehicle {
            let Some(terrain) = self.data.terrain_at(position.0, position.1) else {
                return false;
            };
            match index {
                0 if !terrain.boat_pass => return false,
                1 if !terrain.ship_pass => return false,
                2 => return terrain.airship_pass,
                _ => {}
            }
        }
        if let Some(tile) = self.tile_event(position, self_id).filter(|tile| *tile > 0) {
            let passage = self.passage(tile);
            if passage & ABOVE_HERO_BIT == 0 {
                return vehicle.is_none() && passage & bit != 0;
            }
        }
        if vehicle.is_some() {
            let tile = self.data.upper[(position.1 * self.data.width + position.0) as usize];
            return tile
                .checked_sub(10000)
                .is_some_and(|tile| self.passage(u32::from(tile)) & ABOVE_HERO_BIT != 0);
        }
        self.data.passable_dir(position.0, position.1, bit)
    }

    fn tile_event(&self, position: (i32, i32), self_id: u32) -> Option<u32> {
        self.events
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
            .map(|(_, tile)| tile)
    }
}
