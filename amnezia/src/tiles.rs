//! RPG Maker 2000 tile-id → chipset source-rect mapping.
//!
//! Each map layer stores u16 tile ids; this converts an id to its 16×16 source
//! rectangle in the 480×256 chipset image. Lower-layer autotile assembly lives
//! in [`autotile`]; this module keeps the charset/upper-layer mappings and the
//! passability rules.

mod autotile;
mod water;

pub use autotile::{lower_render, LowerRender, Quarter, QUARTER};
pub use water::{
    block_c_source, is_ab_water, is_block_c, water_quarters, BLOCK_C_FRAMES, WATER_FRAMES,
};

/// Size of one tile in pixels.
pub const TILE: f32 = 16.0;

/// CharSet facing rows (the `dir_row` of [`charset_source`]).
pub const DIR_UP: u32 = 0;
pub const DIR_RIGHT: u32 = 1;
pub const DIR_DOWN: u32 = 2;
pub const DIR_LEFT: u32 = 3;

/// Vertical offset so a 24×32 character's feet sit on the tile it occupies
/// (RM2000 aligns the sprite's bottom with the tile's bottom).
pub const CHAR_Y_OFFSET: f32 = (CHAR_H - TILE) / 2.0;

/// Draw depth for a character (hero or event NPC) at tile row `tile_y`. RM2000
/// y-sorts dynamic characters: one lower on screen (larger `tile_y`) draws in
/// front. The result stays in `[2.0, 4.0)` for any map up to 199 tiles tall, so
/// characters sit above the tile layers (z 0/1) yet below "above hero" upper
/// tiles (z 4), which keep occluding roofs and treetops.
pub fn character_z(tile_y: i32) -> f32 {
    2.0 + (tile_y.clamp(0, 199) as f32) * 0.01
}

/// Source rectangle top-left for an upper-layer tile id, or `None` when the
/// tile is empty (id 10000 or below) and must not be drawn.
pub fn upper_source(id: u16) -> Option<(f32, f32)> {
    if id <= 10000 {
        return None;
    }
    let i = id - 10000;
    let (col, row) = if i < 48 {
        (18 + i % 6, 8 + i / 6)
    } else {
        let j = i - 48;
        (24 + j % 6, j / 6)
    };
    Some((f32::from(col) * TILE, f32::from(row) * TILE))
}

/// Width and height of one CharSet sprite cell in pixels.
pub const CHAR_W: f32 = 24.0;
pub const CHAR_H: f32 = 32.0;

/// Source rectangle top-left (charset pixels) for a character sprite:
/// `char_index` selects one of the 8 blocks (4×2), `dir_row` the facing row
/// (Up=0, Right=1, Down=2, Left=3), `frame_col` the walk frame (0/1/2).
pub fn charset_source(char_index: u32, dir_row: u32, frame_col: u32) -> (f32, f32) {
    let block_x = (char_index % 4) * 72;
    let block_y = (char_index / 4) * 128;
    let x = block_x + frame_col * 24;
    let y = block_y + dir_row * 32;
    (x as f32, y as f32)
}

fn passages_lower_index(id: u16) -> Option<usize> {
    if id < 3000 {
        Some((id / 1000) as usize)
    } else if (3000..3150).contains(&id) {
        Some((id - 3000) as usize / 50 + 3)
    } else if (4000..4600).contains(&id) {
        Some((id - 4000) as usize / 50 + 6)
    } else if (5000..=5143).contains(&id) {
        Some((id - 5000) as usize + 18)
    } else {
        None
    }
}

/// The `passages_up` bit (RM2000 "above hero" / star / priority) that draws an
/// upper-layer tile above the hero instead of at or below it.
const ABOVE_HERO_BIT: u8 = 0x10;

/// Whether an upper-layer tile is flagged "above hero" (roof tops, tree tops,
/// tall-object tops): bit [`ABOVE_HERO_BIT`] of its `passages_up` byte. Such a
/// tile renders above the hero so the hero walks behind it; ordinary upper tiles
/// render at or below the hero. The empty upper tile (`id <= 10000`) and ids
/// past the array are never above-hero. Indexes `passages_up` the same way
/// [`passable`] and [`upper_source`] do (`id - 10000`).
pub fn above_hero(upper_id: u16, passages_up: &[u8]) -> bool {
    if upper_id <= 10000 {
        return false;
    }
    passages_up
        .get((upper_id - 10000) as usize)
        .is_some_and(|byte| byte & ABOVE_HERO_BIT != 0)
}

/// The `passages` "wall" bit; a BLOCK_D autotile with this set is a wall whose
/// walk-on shapes (edges/thresholds) the hero can still cross.
const WALL_BIT: u8 = 0x20;

/// Whether the hero can stand on a lower-layer tile. Mirrors EasyRPG's
/// `Game_Map::IsPassableLowerTile`: a BLOCK_D autotile (ids 4000..4600) with the
/// wall bit set is passable on its walk-on shapes (`(id-4000) % 50` in the
/// edge/threshold set), regardless of direction bits; every other tile is
/// passable when any of its four direction bits is set.
fn lower_passable(lower_id: u16, passages_down: &[u8]) -> bool {
    let byte = passages_lower_index(lower_id)
        .and_then(|i| passages_down.get(i))
        .copied()
        .unwrap_or(0x0F);
    if (4000..4600).contains(&lower_id) {
        let shape = (lower_id - 4000) % 50;
        if byte & WALL_BIT != 0 && matches!(shape, 20..=23 | 33..=37 | 42 | 43 | 45 | 46) {
            return true;
        }
    }
    byte & 0x0F != 0
}

/// Whether the hero can stand on a cell with the given lower/upper tile ids, per
/// the active chipset's passability arrays (`passages_down` 162 bytes,
/// `passages_up` 144). The upper layer decides first: a solid upper tile blocks;
/// a passable non-"above hero" upper tile is walkable; an "above hero" upper
/// tile (and the empty upper tile) defers to the lower tile.
pub fn passable(lower_id: u16, upper_id: u16, passages_down: &[u8], passages_up: &[u8]) -> bool {
    let lower_ok = lower_passable(lower_id, passages_down);
    if upper_id <= 10000 {
        return lower_ok;
    }
    let upper = passages_up.get((upper_id - 10000) as usize).copied().unwrap_or(0x0F);
    if upper & 0x0F == 0 {
        return false;
    }
    if upper & ABOVE_HERO_BIT == 0 {
        return true;
    }
    lower_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_empty_is_none() {
        assert_eq!(upper_source(10000), None);
    }

    #[test]
    fn upper_tile_rect() {
        assert_eq!(upper_source(10001), Some((304.0, 128.0)));
    }

    #[test]
    fn charset_hero_down_idle() {
        assert_eq!(charset_source(0, 2, 1), (24.0, 64.0));
    }

    #[test]
    fn character_z_sorts_by_row_and_stays_below_above_hero() {
        assert!(character_z(5) < character_z(6)); // lower on screen draws in front
        assert!(character_z(0) >= 2.0); // above the tile layers
        assert!(character_z(199) < 4.0); // below "above hero" upper tiles (z 4)
    }

    #[test]
    fn charset_block_origin_for_index_five() {
        assert_eq!(charset_source(5, 0, 0), (72.0, 128.0));
    }

    #[test]
    fn above_hero_reads_the_0x10_bit() {
        let mut up = vec![0x0F; 144];
        up[5] = 0x1F; // 0x0F | 0x10: passable star tile drawn above the hero
        up[6] = 0x0F; // ordinary passable upper tile, at/below the hero
        up[7] = 0x10; // above-hero even with no direction bits set
        assert!(above_hero(10005, &up));
        assert!(!above_hero(10006, &up));
        assert!(above_hero(10007, &up));
        assert!(!above_hero(10000, &up)); // the empty upper tile is never above
        assert!(!above_hero(9999, &up)); // below the upper-layer id range
        assert!(!above_hero(10144, &up)); // index past the array defaults to not-above
    }

    #[test]
    fn passable_grass_water_and_upper_obstacle() {
        let mut down = vec![0x0F; 162];
        down[0] = 0x00;
        let mut up = vec![0x0F; 144];
        up[30] = 0x00;
        assert!(passable(4050, 10000, &down, &up));
        assert!(!passable(40, 10000, &down, &up));
        assert!(!passable(4000, 10030, &down, &up));
    }

    #[test]
    fn wall_autotile_walk_on_shapes_are_passable() {
        let mut down = vec![0x0F; 162];
        down[17] = 0x30; // BLOCK_D autotile #11: wall bit set, no direction bits
        let up = vec![0x0F; 144];
        // shape 0 (id 4550) is a solid wall corner — impassable
        assert!(!passable(4550, 10000, &down, &up));
        // shapes 20 (id 4570) and 33 (id 4583) are walk-on thresholds — passable
        assert!(passable(4570, 10000, &down, &up));
        assert!(passable(4583, 10000, &down, &up));
        // without the wall bit, a shape with no direction bits stays impassable
        down[17] = 0x00;
        assert!(!passable(4570, 10000, &down, &up));
    }
}
