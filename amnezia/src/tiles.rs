//! RPG Maker 2000 tile-id → chipset source-rect mapping.
//!
//! Each map layer stores u16 tile ids; this converts an id to the top-left
//! pixel of its 16×16 source rectangle in the 480×256 chipset image. This is a
//! first-pass "representative tile" mapping: autotile ids resolve to a single
//! representative tile rather than a neighbour-assembled shape.

/// Size of one tile in pixels.
pub const TILE: f32 = 16.0;

/// Source rectangle top-left (chipset pixels) for a lower-layer tile id.
pub fn lower_source(id: u16) -> (f32, f32) {
    let (col, row): (u16, u16) = if id < 4000 {
        (0, 4)
    } else if id < 5000 {
        let block = (id - 4000) / 50;
        let (bx, by) = if block < 4 {
            ((block % 2) * 3, 8 + (block / 2) * 4)
        } else {
            (6 + (block % 2) * 3, ((block - 4) / 2) * 4)
        };
        (bx + 1, by + 2)
    } else {
        let i = id - 5000;
        if i < 96 {
            (12 + i % 6, i / 6)
        } else {
            let j = i - 96;
            (18 + j % 6, j / 6)
        }
    };
    (f32::from(col) * TILE, f32::from(row) * TILE)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_autotile_centre() {
        assert_eq!(lower_source(4050), (64.0, 160.0));
        assert_eq!(lower_source(4054), (64.0, 160.0));
    }

    #[test]
    fn plain_lower_block_e() {
        assert_eq!(lower_source(5000), (192.0, 0.0));
    }

    #[test]
    fn water_representative() {
        assert_eq!(lower_source(25), (0.0, 64.0));
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
    fn charset_block_origin_for_index_five() {
        assert_eq!(charset_source(5, 0, 0), (72.0, 128.0));
    }
}
