use super::MapData;
use crate::tiles::{PASS_ALL, lower_passable, passages_lower_index};
use amnezia_data::TerrainDef;

impl MapData {
    pub(crate) fn terrain_at(&self, x: i32, y: i32) -> Option<&TerrainDef> {
        let (x, y) = self.normalize_tile(x, y);
        let index = if self.contains_tile(x, y) {
            passages_lower_index(self.lower[(y * self.width + x) as usize]).unwrap_or(0)
        } else {
            0
        };
        let id = self.terrain_data.get(index).copied().unwrap_or(1) as u32;
        self.terrains.iter().find(|terrain| terrain.id == id)
    }

    pub(crate) fn airship_passable(&self, x: i32, y: i32) -> bool {
        self.contains_tile(x, y) && self.terrain_at(x, y).is_some_and(|t| t.airship_pass)
    }

    pub(crate) fn airship_landing_tile(&self, x: i32, y: i32) -> bool {
        if !self.contains_tile(x, y) || !self.terrain_at(x, y).is_some_and(|t| t.airship_land) {
            return false;
        }
        let (x, y) = self.normalize_tile(x, y);
        let index = (y * self.width + x) as usize;
        let upper = self.upper[index]
            .checked_sub(10000)
            .and_then(|tile| self.passages_up.get(tile as usize))
            .copied()
            .unwrap_or(0);
        lower_passable(self.lower[index], &self.passages_down, PASS_ALL) && upper & PASS_ALL != 0
    }
}

#[cfg(test)]
mod tests;
