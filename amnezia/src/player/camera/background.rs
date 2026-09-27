use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct BackgroundScroll {
    pub(crate) map_id: u32,
    pub(crate) display: [f32; 2],
    pub(crate) delta: Option<[f32; 2]>,
}

impl BackgroundScroll {
    pub(super) fn valid(&self) -> bool {
        self.display
            .iter()
            .chain(self.delta.iter().flatten())
            .all(|value| value.is_finite())
    }
}

impl CameraPan {
    pub(crate) fn take_background_scroll(&mut self) -> Vec<BackgroundScroll> {
        std::mem::take(&mut self.background_scroll)
    }

    pub(crate) fn background_position(&self, data: &MapData) -> Option<Vec2> {
        let position = self.position?;
        Some(self.canonical_display().unwrap_or_else(|| {
            let corner = Vec2::from(data.tile_center(0, 0)) + Vec2::new(-8.0, 8.0);
            (position - corner) * Vec2::new(1.0, -1.0) - Vec2::new(160.0, 120.0)
        }))
    }
}

#[cfg(test)]
mod tests;
