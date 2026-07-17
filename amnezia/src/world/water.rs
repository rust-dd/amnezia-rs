//! Water tile animation: the shared animation clock and the per-frame system
//! that re-samples every animated `BLOCK_A`/`BLOCK_B` water quarter and
//! `BLOCK_C` waterfall cell against its current chipset source rect.

use crate::tiles;
use bevy::prelude::*;

/// Marks one 8×8 quarter of an animated `BLOCK_A`/`BLOCK_B` water tile so
/// [`animate_water`] can re-sample its scrolling source column each frame.
#[derive(Component)]
pub(super) struct WaterQuarter {
    pub(super) id: u16,
    pub(super) quarter: usize,
}

/// Marks a whole-cell `BLOCK_C` animated tile (waterfalls) for [`animate_water`].
#[derive(Component)]
pub(super) struct WaterCell {
    pub(super) id: u16,
}

/// The shared water-animation clock: each timer tick advances `step`, which
/// drives the `BLOCK_A`/`BLOCK_B` column cycle and the `BLOCK_C` row cycle.
#[derive(Resource)]
pub(super) struct WaterAnim {
    timer: Timer,
    step: u32,
}

impl Default for WaterAnim {
    fn default() -> Self {
        // RM2000 water animates gently; ~0.4 s per step gives the classic cadence
        // (we do not parse the chipset's `animation_speed` database field).
        Self {
            timer: Timer::from_seconds(0.4, TimerMode::Repeating),
            step: 0,
        }
    }
}

/// Advance the water clock and re-point every animated water sprite at its
/// current frame: `BLOCK_A`/`BLOCK_B` quarters scroll one chipset column per step
/// ([`tiles::WATER_FRAMES`]); `BLOCK_C` cells step down one animation row.
pub(super) fn animate_water(
    time: Res<Time>,
    mut anim: ResMut<WaterAnim>,
    mut quarters: Query<(&WaterQuarter, &mut Sprite), Without<WaterCell>>,
    mut cells: Query<(&WaterCell, &mut Sprite), Without<WaterQuarter>>,
) {
    if !anim.timer.tick(time.delta()).just_finished() {
        return;
    }
    anim.step = anim.step.wrapping_add(1);
    let ab_frame = tiles::WATER_FRAMES[(anim.step % tiles::WATER_FRAMES.len() as u32) as usize];
    for (water, mut sprite) in &mut quarters {
        let src = tiles::water_quarters(water.id, ab_frame)[water.quarter].src;
        sprite.rect = Some(Rect::new(
            src.0,
            src.1,
            src.0 + tiles::QUARTER,
            src.1 + tiles::QUARTER,
        ));
    }
    let c_frame = (anim.step % u32::from(tiles::BLOCK_C_FRAMES)) as u16;
    for (water, mut sprite) in &mut cells {
        let src = tiles::block_c_source(water.id, c_frame);
        sprite.rect = Some(Rect::new(
            src.0,
            src.1,
            src.0 + tiles::TILE,
            src.1 + tiles::TILE,
        ));
    }
}
