//! RM2000 animated water autotile assembly (`BLOCK_A`/`BLOCK_B`/`BLOCK_C`),
//! mirroring EasyRPG/Player `src/tilemap_layer.cpp`: `GenerateAutotileAB` plus
//! its `BlockA_Subtiles_IDS` table for the quarter-assembled `BLOCK_A`/`BLOCK_B`
//! tiles, and the `Draw` block-C branch for the whole-cell `BLOCK_C` tiles.
//!
//! The water source occupies the chipset's top-left 6×8 tile region (columns
//! 0..=5, rows 0..=7): columns 0..=2 hold the A1 (grass) water with its B coast
//! and deep-ocean rows below (rows 4..=7); columns 3..=5 hold the A2 (snow)
//! water with the C animated tiles below. Assembly matches `BLOCK_D`: a shape
//! selects, per 8×8 quarter, which template cell supplies that corner. The
//! animation `frame` (0..=2) shifts the sampled column, which is a pure
//! horizontal scroll of the source (columns stay within 0..=5).

use super::{Quarter, QUARTER, TILE};

/// First id of `BLOCK_C`; ids below it are the quarter-assembled
/// `BLOCK_A`/`BLOCK_B` water. Matches EasyRPG's `BLOCK_C` (`map_data.h`).
pub const BLOCK_C: u16 = 3000;

/// One past the last `BLOCK_C` id (3 tiles × 50-shape stride). EasyRPG's
/// `BLOCK_C_END`.
pub const BLOCK_C_END: u16 = 3150;

/// The four quarters of an assembled `BLOCK_A`/`BLOCK_B` water tile (id in
/// `0..3000`) at animation `frame`, following EasyRPG's `GenerateAutotileAB`.
/// `block = id / 1000` picks the water pair (0: grass + coast, 1: snow + coast,
/// 2: grass + deep ocean); `b_subtile` selects the coast/ocean edge shape and
/// `a_subtile` the shore shape, each quarter copying an 8×8 corner of one cell.
pub fn water_quarters(id: u16, frame: u16) -> [Quarter; 4] {
    let block = id / 1000;
    let within = id - block * 1000;
    let b_subtile = within / 50;
    let a_subtile = usize::from(within - b_subtile * 50);

    // Ids past the shape tables are invalid in RM2000; fall back to plain deep
    // water (the same cell id 0 assembles) rather than index out of bounds.
    if a_subtile >= BLOCK_A_SUBTILES.len() || b_subtile >= 16 {
        return quarters_from_cells([[(frame, 4); 2]; 2]);
    }

    // Corner order is EasyRPG's `j * 2 + i`: 0 top-left, 1 top-right, 2
    // bottom-left, 3 bottom-right, which is also the bit index into `b_subtile`.
    let mut cells = [[(0u16, 0u16); 2]; 2];
    for corner in 0..4usize {
        let (j, i) = (corner / 2, corner % 2);
        let a = BLOCK_A_SUBTILES[a_subtile][j][i];
        cells[j][i] = if a >= 0 {
            // A-block quarter: the snow pair (block 1) sits three columns right;
            // the row comes straight from the shape table.
            (frame + if block == 1 { 3 } else { 0 }, a as u16)
        } else {
            // B-block quarter: coast (rows 4/5) or, for deep ocean, rows 6/7.
            let mut t = (b_subtile >> corner) & 1;
            if block == 2 {
                t ^= 3;
            }
            (frame, 4 + t)
        };
    }

    // Where a shore shape and a coast edge coincide, the coast water overrides
    // that shore quarter's corner (EasyRPG's third combining pass).
    if b_subtile != 0 && a_subtile != 0 {
        for corner in 0..4usize {
            let mut t = (b_subtile >> corner) & 1;
            if block == 2 {
                t *= 2;
            }
            if t != 0 {
                cells[corner / 2][corner % 2] = (frame, 4 + t);
            }
        }
    }

    quarters_from_cells(cells)
}

/// Build the four [`Quarter`]s from their per-corner `(col, row)` chipset cells,
/// each copying its own `[j][i]` 8×8 sub-corner (EasyRPG's `GenerateAutotiles`
/// blit: `src = col*TILE + i*QUARTER, row*TILE + j*QUARTER`).
fn quarters_from_cells(cells: [[(u16, u16); 2]; 2]) -> [Quarter; 4] {
    let mut quarters = [Quarter::default(); 4];
    for corner in 0..4usize {
        let (col, row) = cells[corner / 2][corner % 2];
        let (qx, qy) = ((corner % 2) as u16, (corner / 2) as u16);
        quarters[corner] = Quarter {
            dst: (f32::from(qx) * QUARTER, f32::from(qy) * QUARTER),
            src: (
                f32::from(col) * TILE + f32::from(qx) * QUARTER,
                f32::from(row) * TILE + f32::from(qy) * QUARTER,
            ),
        };
    }
    quarters
}

/// Top-left chipset pixel of a whole-cell `BLOCK_C` animated tile (ids
/// `3000..3150`) at animation `frame` (0..=3), per EasyRPG's `Draw` block-C
/// branch: `col = 3 + (id - 3000) / 50`, `row = 4 + frame`.
pub fn block_c_source(id: u16, frame: u16) -> (f32, f32) {
    let col = 3 + (id - BLOCK_C) / 50;
    let row = 4 + frame;
    (f32::from(col) * TILE, f32::from(row) * TILE)
}

/// EasyRPG's `BlockA_Subtiles_IDS[47][2][2]`: for water shape `a_subtile`, the
/// A-block source row (0..=3) supplying each `[j][i]` quarter, or `-1` when that
/// quarter is supplied by the B block (coast/ocean) instead.
#[rustfmt::skip]
const BLOCK_A_SUBTILES: [[[i8; 2]; 2]; 47] = [
    [[-1, -1], [-1, -1]],
    [[ 3, -1], [-1, -1]],
    [[-1,  3], [-1, -1]],
    [[ 3,  3], [-1, -1]],
    [[-1, -1], [-1,  3]],
    [[ 3, -1], [-1,  3]],
    [[-1,  3], [-1,  3]],
    [[ 3,  3], [-1,  3]],
    [[-1, -1], [ 3, -1]],
    [[ 3, -1], [ 3, -1]],
    [[-1,  3], [ 3, -1]],
    [[ 3,  3], [ 3, -1]],
    [[-1, -1], [ 3,  3]],
    [[ 3, -1], [ 3,  3]],
    [[-1,  3], [ 3,  3]],
    [[ 3,  3], [ 3,  3]],
    [[ 1, -1], [ 1, -1]],
    [[ 1,  3], [ 1, -1]],
    [[ 1, -1], [ 1,  3]],
    [[ 1,  3], [ 1,  3]],
    [[ 2,  2], [-1, -1]],
    [[ 2,  2], [-1,  3]],
    [[ 2,  2], [ 3, -1]],
    [[ 2,  2], [ 3,  3]],
    [[-1,  1], [-1,  1]],
    [[-1,  1], [ 3,  1]],
    [[ 3,  1], [-1,  1]],
    [[ 3,  1], [ 3,  1]],
    [[-1, -1], [ 2,  2]],
    [[ 3, -1], [ 2,  2]],
    [[-1,  3], [ 2,  2]],
    [[ 3,  3], [ 2,  2]],
    [[ 1,  1], [ 1,  1]],
    [[ 2,  2], [ 2,  2]],
    [[ 0,  2], [ 1, -1]],
    [[ 0,  2], [ 1,  3]],
    [[ 2,  0], [-1,  1]],
    [[ 2,  0], [ 3,  1]],
    [[-1,  1], [ 2,  0]],
    [[ 3,  1], [ 2,  0]],
    [[ 1, -1], [ 0,  2]],
    [[ 1,  3], [ 0,  2]],
    [[ 0,  0], [ 1,  1]],
    [[ 0,  2], [ 0,  2]],
    [[ 1,  1], [ 0,  0]],
    [[ 2,  0], [ 2,  0]],
    [[ 0,  0], [ 0,  0]],
];

#[cfg(test)]
mod tests {
    use super::*;

    fn srcs(id: u16, frame: u16) -> [(f32, f32); 4] {
        water_quarters(id, frame).map(|q| q.src)
    }

    #[test]
    fn plain_water_id_zero_is_the_deep_water_cell_split_in_four() {
        // a_subtile 0 is all-B: the four 8×8 corners of the (col 0, row 4) cell,
        // matching what the old whole-cell fallback drew at (0, 64).
        assert_eq!(srcs(0, 0), [(0.0, 64.0), (8.0, 64.0), (0.0, 72.0), (8.0, 72.0)]);
        let q = water_quarters(0, 0);
        assert_eq!(q.map(|x| x.dst), [(0.0, 0.0), (8.0, 0.0), (0.0, 8.0), (8.0, 8.0)]);
    }

    #[test]
    fn grass_shore_pulls_top_from_the_a_block() {
        // id 3: top quarters from A row 3 (col 0), bottom from B coast (row 4).
        assert_eq!(srcs(3, 0), [(0.0, 48.0), (8.0, 48.0), (0.0, 72.0), (8.0, 72.0)]);
    }

    #[test]
    fn snow_block_one_shifts_the_a_quarters_three_columns_right() {
        // id 1003: A quarters come from the snow columns (col 3), B from col 0.
        assert_eq!(srcs(1003, 0), [(48.0, 48.0), (56.0, 48.0), (0.0, 72.0), (8.0, 72.0)]);
    }

    #[test]
    fn deep_ocean_block_two_maps_b_to_rows_six_and_seven() {
        // id 2050: block 2 B edges land on the deep-ocean rows 6/7 (cols 0).
        assert_eq!(srcs(2050, 0), [(0.0, 96.0), (8.0, 112.0), (0.0, 120.0), (8.0, 120.0)]);
    }

    #[test]
    fn animation_frame_scrolls_the_source_one_column_right() {
        // Every quarter's column advances by `frame`, a pure horizontal scroll.
        for id in [0u16, 3, 108, 1003, 2050] {
            let base = srcs(id, 0);
            for frame in 1..=2u16 {
                let f = srcs(id, frame);
                for (b, a) in base.iter().zip(f.iter()) {
                    assert_eq!(*a, (b.0 + f32::from(frame) * TILE, b.1));
                }
            }
        }
    }

    #[test]
    fn block_c_is_a_whole_animated_cell() {
        assert_eq!(block_c_source(3000, 0), (48.0, 64.0));
        assert_eq!(block_c_source(3050, 0), (64.0, 64.0));
        assert_eq!(block_c_source(3100, 0), (80.0, 64.0));
        // frame steps the row down through the four animation rows (4..=7).
        assert_eq!(block_c_source(3000, 3), (48.0, 112.0));
    }
}
