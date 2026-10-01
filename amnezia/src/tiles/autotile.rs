//! Lower-layer assembly from EasyRPG `tilemap_layer.cpp` and `map_data.h`.
//! ID ranges: A/B water `0..3000`, C animation `3000..3150`, D terrain/walls
//! `4000..4600` (12 sets × 50 shapes), E static tiles `5000..=5143`.
//! A/B/D shapes assemble four 8×8 corners; sampling one whole template cell
//! loses the edge/corner pattern.

use super::TILE;
use super::water;

const BLOCK_D: u16 = 4000;
const BLOCK_D_END: u16 = 4600;
const BLOCK_E: u16 = 5000;
const BLOCK_E_END: u16 = 5144;

/// Side of one autotile quarter in pixels (a 16×16 tile is four 8×8 quarters).
pub const QUARTER: f32 = TILE / 2.0;

/// One 8×8 quarter of a composed 16×16 lower tile: `dst` is its offset within
/// the tile (`0` or `8` on each axis, image-space y-down) and `src` its
/// top-left in chipset pixels.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Quarter {
    pub dst: (f32, f32),
    pub src: (f32, f32),
}

/// One 16×16 source cell or four assembled 8×8 quarters.
#[derive(Clone, PartialEq, Debug)]
pub enum LowerRender {
    Whole { src: (f32, f32) },
    Quarters([Quarter; 4]),
}

/// The chipset source for a lower-layer tile id: `BLOCK_A`/`BLOCK_B` water and
/// `BLOCK_D` autotiles resolve to four shape-assembled quarters, `BLOCK_C` and
/// `BLOCK_E` to a whole static cell (animation frame 0). Any id past the known
/// blocks falls back to the deep-water cell.
pub fn lower_render(id: u16) -> LowerRender {
    if id < water::BLOCK_C {
        LowerRender::Quarters(water::water_quarters(id, 0))
    } else if (water::BLOCK_C..water::BLOCK_C_END).contains(&id) {
        LowerRender::Whole {
            src: water::block_c_source(id, 0),
        }
    } else if (BLOCK_D..BLOCK_D_END).contains(&id) {
        LowerRender::Quarters(block_d_quarters(id))
    } else if (BLOCK_E..BLOCK_E_END).contains(&id) {
        LowerRender::Whole {
            src: block_e_source(id),
        }
    } else {
        LowerRender::Whole {
            src: (0.0, 4.0 * TILE),
        }
    }
}

/// Top-left chipset cell (in tile units) of terrain autotile `block` (0..11);
/// identical to EasyRPG's `GenerateAutotileD` block origin.
fn block_d_origin(block: u16) -> (u16, u16) {
    if block < 4 {
        ((block % 2) * 3, 8 + (block / 2) * 4)
    } else {
        (6 + (block % 2) * 3, ((block - 4) / 2) * 4)
    }
}

/// The four quarters of a `BLOCK_D` autotile. Quarter `(j, i)` (j: 0 top /
/// 1 bottom, i: 0 left / 1 right) copies the same corner of the template cell
/// `origin + BLOCK_D_SUBTILES[shape][j][i]`.
fn block_d_quarters(id: u16) -> [Quarter; 4] {
    let block = (id - BLOCK_D) / 50;
    let shape = ((id - BLOCK_D) % 50) as usize;
    let (bx, by) = block_d_origin(block);
    let mut quarters = [Quarter::default(); 4];
    for j in 0..2u16 {
        for i in 0..2u16 {
            let [dx, dy] = BLOCK_D_SUBTILES[shape][j as usize][i as usize];
            let col = bx + u16::from(dx);
            let row = by + u16::from(dy);
            quarters[(j * 2 + i) as usize] = Quarter {
                dst: (f32::from(i) * QUARTER, f32::from(j) * QUARTER),
                src: (
                    f32::from(col) * TILE + f32::from(i) * QUARTER,
                    f32::from(row) * TILE + f32::from(j) * QUARTER,
                ),
            };
        }
    }
    quarters
}

/// Top-left chipset pixel of a static `BLOCK_E` tile (ids 5000..=5143).
fn block_e_source(id: u16) -> (f32, f32) {
    let i = id - BLOCK_E;
    let (col, row) = if i < 96 {
        (12 + i % 6, i / 6)
    } else {
        (18 + (i - 96) % 6, (i - 96) / 6)
    };
    (f32::from(col) * TILE, f32::from(row) * TILE)
}

/// EasyRPG's `BlockD_Subtiles_IDS[50][2][2]`: for each shape, the `(dx, dy)`
/// cell offset (added to the block origin) that supplies each `[j][i]` quarter.
#[rustfmt::skip]
const BLOCK_D_SUBTILES: [[[[u8; 2]; 2]; 2]; 50] = [
    [[[1, 2], [1, 2]], [[1, 2], [1, 2]]],
    [[[2, 0], [1, 2]], [[1, 2], [1, 2]]],
    [[[1, 2], [2, 0]], [[1, 2], [1, 2]]],
    [[[2, 0], [2, 0]], [[1, 2], [1, 2]]],
    [[[1, 2], [1, 2]], [[1, 2], [2, 0]]],
    [[[2, 0], [1, 2]], [[1, 2], [2, 0]]],
    [[[1, 2], [2, 0]], [[1, 2], [2, 0]]],
    [[[2, 0], [2, 0]], [[1, 2], [2, 0]]],
    [[[1, 2], [1, 2]], [[2, 0], [1, 2]]],
    [[[2, 0], [1, 2]], [[2, 0], [1, 2]]],
    [[[1, 2], [2, 0]], [[2, 0], [1, 2]]],
    [[[2, 0], [2, 0]], [[2, 0], [1, 2]]],
    [[[1, 2], [1, 2]], [[2, 0], [2, 0]]],
    [[[2, 0], [1, 2]], [[2, 0], [2, 0]]],
    [[[1, 2], [2, 0]], [[2, 0], [2, 0]]],
    [[[2, 0], [2, 0]], [[2, 0], [2, 0]]],
    [[[0, 2], [0, 2]], [[0, 2], [0, 2]]],
    [[[0, 2], [2, 0]], [[0, 2], [0, 2]]],
    [[[0, 2], [0, 2]], [[0, 2], [2, 0]]],
    [[[0, 2], [2, 0]], [[0, 2], [2, 0]]],
    [[[1, 1], [1, 1]], [[1, 1], [1, 1]]],
    [[[1, 1], [1, 1]], [[1, 1], [2, 0]]],
    [[[1, 1], [1, 1]], [[2, 0], [1, 1]]],
    [[[1, 1], [1, 1]], [[2, 0], [2, 0]]],
    [[[2, 2], [2, 2]], [[2, 2], [2, 2]]],
    [[[2, 2], [2, 2]], [[2, 0], [2, 2]]],
    [[[2, 0], [2, 2]], [[2, 2], [2, 2]]],
    [[[2, 0], [2, 2]], [[2, 0], [2, 2]]],
    [[[1, 3], [1, 3]], [[1, 3], [1, 3]]],
    [[[2, 0], [1, 3]], [[1, 3], [1, 3]]],
    [[[1, 3], [2, 0]], [[1, 3], [1, 3]]],
    [[[2, 0], [2, 0]], [[1, 3], [1, 3]]],
    [[[0, 2], [2, 2]], [[0, 2], [2, 2]]],
    [[[1, 1], [1, 1]], [[1, 3], [1, 3]]],
    [[[0, 1], [0, 1]], [[0, 1], [0, 1]]],
    [[[0, 1], [0, 1]], [[0, 1], [2, 0]]],
    [[[2, 1], [2, 1]], [[2, 1], [2, 1]]],
    [[[2, 1], [2, 1]], [[2, 0], [2, 1]]],
    [[[2, 3], [2, 3]], [[2, 3], [2, 3]]],
    [[[2, 0], [2, 3]], [[2, 3], [2, 3]]],
    [[[0, 3], [0, 3]], [[0, 3], [0, 3]]],
    [[[0, 3], [2, 0]], [[0, 3], [0, 3]]],
    [[[0, 1], [2, 1]], [[0, 1], [2, 1]]],
    [[[0, 1], [0, 1]], [[0, 3], [0, 3]]],
    [[[0, 3], [2, 3]], [[0, 3], [2, 3]]],
    [[[2, 1], [2, 1]], [[2, 3], [2, 3]]],
    [[[0, 1], [2, 1]], [[0, 3], [2, 3]]],
    [[[1, 2], [1, 2]], [[1, 2], [1, 2]]],
    [[[1, 2], [1, 2]], [[1, 2], [1, 2]]],
    [[[0, 0], [0, 0]], [[0, 0], [0, 0]]],
];

#[cfg(test)]
mod tests {
    use super::*;

    fn srcs(id: u16) -> [(f32, f32); 4] {
        match lower_render(id) {
            LowerRender::Quarters(q) => [q[0].src, q[1].src, q[2].src, q[3].src],
            LowerRender::Whole { .. } => panic!("expected quarters for {id}"),
        }
    }

    #[test]
    fn wall_block_origin_matches_easyrpg() {
        assert_eq!(block_d_origin(11), (9, 12));
        assert_eq!(block_d_origin(10), (6, 12));
        assert_eq!(block_d_origin(1), (3, 8));
    }

    #[test]
    fn wall_center_shape_samples_the_center_cell() {
        let dst: Vec<_> = match lower_render(4570) {
            LowerRender::Quarters(q) => q.iter().map(|x| x.dst).collect(),
            _ => panic!(),
        };
        assert_eq!(dst, vec![(0.0, 0.0), (8.0, 0.0), (0.0, 8.0), (8.0, 8.0)]);
        assert_eq!(
            srcs(4570),
            [
                (160.0, 208.0),
                (168.0, 208.0),
                (160.0, 216.0),
                (168.0, 216.0)
            ]
        );
    }

    #[test]
    fn wall_fill_shape_zero_is_the_dark_interior_cell() {
        assert_eq!(
            srcs(4550),
            [
                (160.0, 224.0),
                (168.0, 224.0),
                (160.0, 232.0),
                (168.0, 232.0)
            ]
        );
    }

    #[test]
    fn wall_corner_shape_pulls_one_edge_quarter() {
        assert_eq!(
            srcs(4551),
            [
                (176.0, 192.0),
                (168.0, 224.0),
                (160.0, 232.0),
                (168.0, 232.0)
            ]
        );
    }

    #[test]
    fn terrain_block_one_fill_matches_legacy_representative() {
        assert_eq!(srcs(4050)[0], (64.0, 160.0));
        assert_eq!(srcs(4054)[0], (64.0, 160.0));
    }

    #[test]
    fn static_block_e_is_a_whole_cell() {
        assert_eq!(lower_render(5000), LowerRender::Whole { src: (192.0, 0.0) });
        assert_eq!(lower_render(5001), LowerRender::Whole { src: (208.0, 0.0) });
        assert_eq!(
            lower_render(5015),
            LowerRender::Whole { src: (240.0, 32.0) }
        );
    }

    #[test]
    fn water_block_a_assembles_into_quarters() {
        // Plain water assembles all four corners of the deep-water cell at (0, 64).
        assert_eq!(
            srcs(0),
            [(0.0, 64.0), (8.0, 64.0), (0.0, 72.0), (8.0, 72.0)]
        );
        // A shore shape pulls its top quarters from a different template row.
        assert_eq!(
            srcs(3),
            [(0.0, 48.0), (8.0, 48.0), (0.0, 72.0), (8.0, 72.0)]
        );
    }

    #[test]
    fn water_block_c_is_a_whole_cell() {
        assert_eq!(lower_render(3000), LowerRender::Whole { src: (48.0, 64.0) });
        assert_eq!(lower_render(3100), LowerRender::Whole { src: (80.0, 64.0) });
    }
}
