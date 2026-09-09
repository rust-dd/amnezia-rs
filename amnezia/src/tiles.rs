//! RPG Maker 2000 tile-id → chipset source-rect mapping.
//!
//! Each map layer stores u16 tile ids; this converts an id to its 16×16 source
//! rectangle in the 480×256 chipset image. Lower-layer autotile assembly lives
//! in [`autotile`]; this module keeps the charset/upper-layer mappings and the
//! passability rules.

mod autotile;
mod water;

pub use autotile::{LowerRender, QUARTER, Quarter, lower_render};
pub use water::{
    BLOCK_C_FRAMES, WATER_FRAMES, block_c_source, is_ab_water, is_block_c, water_quarters,
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

/// World draw-Z of the normal ground layer (lower tiles with no star/wall
/// attribute): the bottom of the stack.
pub const Z_GROUND: f32 = 0.0;
/// World draw-Z of normal upper-layer tiles — above the ground, below the hero.
pub const Z_UPPER: f32 = 1.0;
/// Base draw-Z of below-hero-layer event NPCs (page layer 0): above the tile
/// layers, below the hero band.
const Z_EVENTS_BELOW: f32 = 2.0;
/// Base draw-Z of the hero and same-layer event NPCs (page layer 1): the
/// y-sorted "Priority_Player" band.
const Z_SAME: f32 = 4.0;
/// World draw-Z of "above hero" tiles — lower tiles carrying the star or wall
/// attribute plus star upper tiles — which occlude the hero (roofs, treetops,
/// wall tops).
pub const Z_TILE_ABOVE: f32 = 6.0;
/// Base draw-Z of above-hero-layer event NPCs (page layer 2): above even the
/// "above hero" tiles, matching EasyRPG's `Priority_EventsAbove` > `TilesetAbove`.
const Z_EVENTS_ABOVE: f32 = 7.0;

/// The per-row y-sort bias added within a dynamic band: one lower on screen
/// (larger `tile_y`) draws in front. Clamped so any map up to 199 tiles tall
/// keeps its band's 2.0 width without spilling into the next band.
fn row_bias(tile_y: i32) -> f32 {
    (tile_y.clamp(0, 199) as f32) * 0.01
}

/// Draw depth for the hero (or any same-layer character) at tile row `tile_y`,
/// y-sorted within the "Priority_Player" band.
pub fn character_z(tile_y: i32) -> f32 {
    Z_SAME + row_bias(tile_y)
}

/// Draw depth for an event NPC at tile row `tile_y`, placed in the band for its
/// page `layer` (0 below the hero, 1 same as the hero, 2 above the hero and the
/// "above hero" tiles) — RM2000 draws events relative to the hero by page layer.
pub fn character_z_layer(tile_y: i32, layer: u32) -> f32 {
    let base = match layer {
        0 => Z_EVENTS_BELOW,
        2 => Z_EVENTS_ABOVE,
        _ => Z_SAME,
    };
    base + row_bias(tile_y)
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

pub(crate) fn character_alpha(level: u8) -> f32 {
    f32::from((8 - u16::from(level.min(7))) * 32 - 1) / 255.0
}

/// Source rectangle top-left (charset pixels) for a character sprite:
/// `char_index` selects one of the 8 blocks (4×2), `dir_row` the facing row
/// (Up=0, Right=1, Down=2, Left=3). Animation state 3 reuses the middle frame.
pub fn charset_source(char_index: u32, dir_row: u32, frame_col: u32) -> (f32, f32) {
    let block_x = (char_index % 4) * 72;
    let block_y = (char_index / 4) * 128;
    let frame_col = if frame_col >= 3 { 1 } else { frame_col };
    let x = block_x + frame_col * 24;
    let y = block_y + dir_row * 32;
    (x as f32, y as f32)
}

/// RM2000 passability-byte direction bits (a tile's `passages_down`/`passages_up`
/// byte): the direction of travel each bit permits across the tile edge. These
/// are the passability layout, distinct from the CharSet `DIR_*` facing rows.
pub const PASS_DOWN: u8 = 0x01;
pub const PASS_LEFT: u8 = 0x02;
pub const PASS_RIGHT: u8 = 0x04;
pub const PASS_UP: u8 = 0x08;

/// All four direction bits — a non-directional "standable at all" test.
pub const PASS_ALL: u8 = PASS_DOWN | PASS_LEFT | PASS_RIGHT | PASS_UP;

/// The counter attribute bit of an upper-layer tile's `passages_up` byte.
const COUNTER_BIT: u8 = 0x40;

/// The passability bit for a step from `(fx, fy)` to the adjacent `(tx, ty)`:
/// the direction of travel, per EasyRPG's `GetPassableMask`. Passing this bit to
/// [`passable`] on the tile being left checks it permits exit that way; passing
/// the reverse (`passable_mask(tx, ty, fx, fy)`) on the tile being entered checks
/// it permits entry from the opposite side.
pub fn passable_mask(fx: i32, fy: i32, tx: i32, ty: i32) -> u8 {
    let mut bit = 0;
    if tx > fx {
        bit |= PASS_RIGHT;
    }
    if tx < fx {
        bit |= PASS_LEFT;
    }
    if ty > fy {
        bit |= PASS_DOWN;
    }
    if ty < fy {
        bit |= PASS_UP;
    }
    bit
}

/// Whether the upper-layer tile `upper_id` is a counter (RM2000 `IsCounter`):
/// the counter bit of its `passages_up` byte. An action-triggered event one tile
/// beyond a counter can still be interacted with across it. The empty upper tile
/// (`id <= 10000`) and ids past the array are never counters.
pub fn is_counter(upper_id: u16, passages_up: &[u8]) -> bool {
    if upper_id <= 10000 {
        return false;
    }
    passages_up
        .get((upper_id - 10000) as usize)
        .is_some_and(|byte| byte & COUNTER_BIT != 0)
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

/// Whether a LOWER-layer tile must draw above the hero — roof surfaces, wall
/// tops, treetops, and cliff overhangs are painted on the ground layer yet
/// occlude the hero as it passes behind them. RM2000 (EasyRPG
/// `TilemapLayer::CreateTileCacheAt`) raises a lower tile to the above-hero
/// sublayer when its `passages_down` byte carries the star flag OR the wall flag,
/// so wall faces occlude the hero just as star tiles do. Indexes `passages_down`
/// the way [`passable`] does.
pub fn above_hero_lower(lower_id: u16, passages_down: &[u8]) -> bool {
    passages_lower_index(lower_id)
        .and_then(|i| passages_down.get(i))
        .is_some_and(|byte| byte & (ABOVE_HERO_BIT | WALL_BIT) != 0)
}

/// The `passages` "wall" bit; a BLOCK_D autotile with this set is a wall whose
/// walk-on shapes (edges/thresholds) the hero can still cross.
const WALL_BIT: u8 = 0x20;

/// Whether a lower-layer tile permits passage in the direction(s) `bit`. Mirrors
/// EasyRPG's `Game_Map::IsPassableLowerTile`: a BLOCK_D autotile (ids 4000..4600)
/// with the wall bit set is passable on its walk-on shapes (`(id-4000) % 50` in
/// the edge/threshold set) in any direction; every other tile is passable when
/// its byte has the requested direction bit set. Pass [`PASS_ALL`] for a
/// non-directional "standable at all" test.
fn lower_passable(lower_id: u16, passages_down: &[u8], bit: u8) -> bool {
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
    byte & bit != 0
}

/// Whether a cell with the given lower/upper tile ids permits passage in the
/// direction(s) `bit`, per the active chipset's passability arrays
/// (`passages_down` 162 bytes, `passages_up` 144) — EasyRPG's
/// `Game_Map::IsPassableTile` map-geometry rule. The upper layer decides first: a
/// non-empty upper tile blocks when it lacks `bit`; a passable non-"above hero"
/// upper tile is walkable; an "above hero" upper tile (and the empty upper tile,
/// id `<= 10000`) defers to the lower tile. `bit` is a direction from
/// [`passable_mask`], or [`PASS_ALL`] for a non-directional standability test.
pub fn passable(
    lower_id: u16,
    upper_id: u16,
    passages_down: &[u8],
    passages_up: &[u8],
    bit: u8,
) -> bool {
    if upper_id > 10000 {
        let upper = passages_up
            .get((upper_id - 10000) as usize)
            .copied()
            .unwrap_or(0x0F);
        if upper & bit == 0 {
            return false;
        }
        if upper & ABOVE_HERO_BIT == 0 {
            return true;
        }
    }
    lower_passable(lower_id, passages_down, bit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_transparency_uses_the_original_eight_bit_opacity_steps() {
        for (level, opacity) in [255, 223, 191, 159, 127, 95, 63, 31]
            .into_iter()
            .enumerate()
        {
            assert!((character_alpha(level as u8) * 255.0 - opacity as f32).abs() < 1e-5);
        }
        assert_eq!(character_alpha(255), character_alpha(7));
    }

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
        assert!(character_z(0) >= Z_SAME); // above the below-hero event band
        assert!(character_z(199) < Z_TILE_ABOVE); // below the "above hero" tiles
    }

    #[test]
    fn event_layer_bands_order_below_same_above() {
        // A below-layer event sinks under the hero band; an above-layer event
        // rises above both the hero and the "above hero" tiles; same-layer sits
        // with the hero. Each stays y-sorted within its band.
        assert!(character_z_layer(199, 0) < character_z(0)); // below < same
        assert_eq!(character_z_layer(3, 1), character_z(3)); // layer 1 == hero band
        assert!(character_z_layer(0, 2) > Z_TILE_ABOVE); // above > above-hero tiles
        assert!(character_z_layer(5, 0) < character_z_layer(6, 0)); // y-sorted
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
    fn above_hero_lower_reads_the_star_bit_on_ground_tiles() {
        let mut down = vec![0x0F; 162];
        // BLOCK_E index for id 5075 is (5075-5000)+18 = 93 (a roof surface).
        down[93] = 0x1F; // 0x0F | 0x10: passable ground tile flagged to draw above the hero
        down[94] = 0x0F; // ordinary passable ground, at/below the hero
        assert!(above_hero_lower(5075, &down));
        assert!(!above_hero_lower(5076, &down));
        // A plain grass tile (index 0) with no star bit is never above-hero.
        assert!(!above_hero_lower(0, &down));
    }

    #[test]
    fn above_hero_lower_also_reads_the_wall_bit() {
        let mut down = vec![0x0F; 162];
        // BLOCK_D index for id 4000 is (4000-4000)/50 + 6 = 6 (a wall autotile).
        down[6] = 0x20; // wall bit only, no star: a wall face still occludes the hero
        assert!(above_hero_lower(4000, &down));
        down[6] = 0x0F; // a passable BLOCK_D shape with neither star nor wall stays below
        assert!(!above_hero_lower(4000, &down));
    }

    #[test]
    fn passable_grass_water_and_upper_obstacle() {
        let mut down = vec![0x0F; 162];
        down[0] = 0x00;
        let mut up = vec![0x0F; 144];
        up[30] = 0x00;
        assert!(passable(4050, 10000, &down, &up, PASS_ALL));
        assert!(!passable(40, 10000, &down, &up, PASS_ALL));
        assert!(!passable(4000, 10030, &down, &up, PASS_ALL));
    }

    #[test]
    fn passable_is_directional_on_both_layers() {
        // A lower tile passable only downward (0x01) blocks an upward step but
        // allows a downward one; the empty upper defers to it.
        let mut down = vec![0x0F; 162];
        down[0] = PASS_DOWN;
        let up = vec![0x0F; 144];
        assert!(passable(0, 10000, &down, &up, PASS_DOWN));
        assert!(!passable(0, 10000, &down, &up, PASS_UP));
        // A non-star upper tile decides on its own bits regardless of the lower.
        let mut up2 = vec![0x0F; 144];
        up2[5] = PASS_LEFT; // passable only leftward, not "above hero"
        assert!(passable(0, 10005, &down, &up2, PASS_LEFT));
        assert!(!passable(0, 10005, &down, &up2, PASS_RIGHT));
    }

    #[test]
    fn passable_mask_encodes_the_travel_direction() {
        assert_eq!(passable_mask(2, 2, 3, 2), PASS_RIGHT);
        assert_eq!(passable_mask(2, 2, 1, 2), PASS_LEFT);
        assert_eq!(passable_mask(2, 2, 2, 3), PASS_DOWN);
        assert_eq!(passable_mask(2, 2, 2, 1), PASS_UP);
        // The reverse mask (entering tile) is the opposite bit.
        assert_eq!(passable_mask(3, 2, 2, 2), PASS_LEFT);
    }

    #[test]
    fn is_counter_reads_the_0x40_bit() {
        let mut up = vec![0x0F; 144];
        up[5] = 0x4F; // 0x0F | 0x40: a passable counter tile
        up[6] = 0x0F; // ordinary passable upper tile
        assert!(is_counter(10005, &up));
        assert!(!is_counter(10006, &up));
        assert!(!is_counter(10000, &up)); // the empty upper tile is never a counter
    }

    #[test]
    fn wall_autotile_walk_on_shapes_are_passable() {
        let mut down = vec![0x0F; 162];
        down[17] = 0x30; // BLOCK_D autotile #11: wall bit set, no direction bits
        let up = vec![0x0F; 144];
        // shape 0 (id 4550) is a solid wall corner — impassable
        assert!(!passable(4550, 10000, &down, &up, PASS_ALL));
        // shapes 20 (id 4570) and 33 (id 4583) are walk-on thresholds — passable
        assert!(passable(4570, 10000, &down, &up, PASS_ALL));
        assert!(passable(4583, 10000, &down, &up, PASS_ALL));
        // without the wall bit, a shape with no direction bits stays impassable
        down[17] = 0x00;
        assert!(!passable(4570, 10000, &down, &up, PASS_ALL));
    }
}
