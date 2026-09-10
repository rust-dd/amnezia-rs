use crate::tiles;
use crate::timing::GameFrames;
use bevy::prelude::*;

pub(crate) mod smoke;

#[derive(Component)]
pub(super) struct WaterQuarter {
    pub(super) id: u16,
    pub(super) quarter: usize,
}

#[derive(Component)]
pub(super) struct WaterCell {
    pub(super) id: u16,
}

#[derive(Resource, Default)]
pub(super) struct WaterStyle {
    fast: bool,
    cycle: bool,
}

impl WaterStyle {
    pub(super) fn from_chipset(chipset: &amnezia_data::Chipset) -> Self {
        Self {
            fast: chipset.animation_speed != 0,
            cycle: chipset.animation_type != 0,
        }
    }

    fn frames(&self, time: u32) -> (u16, u16) {
        let step = time / if self.fast { 12 } else { 24 };
        let water = if self.cycle {
            (step % 3) as u16
        } else {
            tiles::WATER_FRAMES[(step % 4) as usize]
        };
        (
            water,
            ((time / 6) % u32::from(tiles::BLOCK_C_FRAMES)) as u16,
        )
    }
}

pub(super) fn animate_water(
    time: Res<GameFrames>,
    style: Res<WaterStyle>,
    mut quarters: Query<(&WaterQuarter, &mut Sprite), Without<WaterCell>>,
    mut cells: Query<(&WaterCell, &mut Sprite), Without<WaterQuarter>>,
) {
    let (ab_frame, c_frame) = style.frames(time.frame);
    for (water, mut sprite) in &mut quarters {
        let src = tiles::water_quarters(water.id, ab_frame)[water.quarter].src;
        let rect = Some(Rect::new(
            src.0,
            src.1,
            src.0 + tiles::QUARTER,
            src.1 + tiles::QUARTER,
        ));
        if sprite.rect != rect {
            sprite.rect = rect;
        }
    }
    for (water, mut sprite) in &mut cells {
        let src = tiles::block_c_source(water.id, c_frame);
        let rect = Some(Rect::new(
            src.0,
            src.1,
            src.0 + tiles::TILE,
            src.1 + tiles::TILE,
        ));
        if sprite.rect != rect {
            sprite.rect = rect;
        }
    }
}

#[cfg(test)]
mod tests;
