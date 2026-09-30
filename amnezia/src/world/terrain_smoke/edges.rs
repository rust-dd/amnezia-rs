use amnezia_data::Map;
use bevy::prelude::*;

fn ledges(map: u32) -> &'static [(u32, u32, u16)] {
    match map {
        220 => &[
            (12, 4, 5109),
            (13, 4, 5110),
            (14, 5, 5109),
            (15, 5, 5109),
            (16, 5, 5110),
        ],
        222 => &[
            (5, 5, 5108),
            (6, 5, 5109),
            (7, 5, 5109),
            (8, 5, 5109),
            (9, 5, 5110),
        ],
        _ => &[],
    }
}

pub(super) fn append(
    id: u32,
    map: &Map,
    chip: &Image,
    offset: (i32, i32),
    pixels: &mut Vec<(u32, u32, [u8; 3])>,
) {
    let before = pixels.len();
    for &(x, y, tile) in ledges(id) {
        let index = (y * map.width + x) as usize;
        assert_eq!(map.lower[index], tile);
        assert_eq!(map.upper[index], 10000);
        // These original static cells bypass the runtime autotile/source mapper.
        let sx = match tile {
            5108 => 288,
            5109 => 304,
            5110 => 320,
            _ => unreachable!(),
        };
        for dy in 0..16 {
            for dx in 0..16 {
                let rgba = chip
                    .get_color_at(sx + dx, 32 + dy)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                assert_eq!(rgba[3], 255);
                let px = offset.0 + (x * 16 + dx) as i32;
                let py = offset.1 + (y * 16 + dy) as i32;
                assert!((0..320).contains(&px) && (0..240).contains(&py));
                pixels.push((px as u32, py as u32, [rgba[0], rgba[1], rgba[2]]));
            }
        }
    }
    if id == 222 {
        // The original cutscene map deliberately leaves its right nine columns blank.
        for y in 0..15 {
            for x in 11..20 {
                let index = (y * map.width + x) as usize;
                assert_eq!(map.lower[index], 5143);
                assert_eq!(map.upper[index], 10000);
                for dy in 0..16 {
                    for dx in 0..16 {
                        let px = offset.0 + (x * 16 + dx) as i32;
                        let py = offset.1 + (y * 16 + dy) as i32;
                        assert!((0..320).contains(&px) && (0..240).contains(&py));
                        pixels.push((px as u32, py as u32, [0; 3]));
                    }
                }
            }
        }
    }
    if pixels.len() > before {
        info!(
            "map {id}: {} original grass-ledge/blank-area pixels scheduled",
            pixels.len() - before
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grass_ledge_probes_use_uncovered_original_static_cells() {
        for id in [220, 222] {
            let map = crate::assets::load_ron::<Map>(&format!(
                "{}/maps/map_{id:04}.ron",
                crate::assets::asset_root()
            ));
            for &(x, y, tile) in ledges(id) {
                let index = (y * map.width + x) as usize;
                assert_eq!(map.lower[index], tile);
                assert_eq!(map.upper[index], 10000);
            }
            if id == 222 {
                for y in 0..map.height {
                    for x in 11..20 {
                        let index = (y * map.width + x) as usize;
                        assert_eq!(map.lower[index], 5143);
                        assert_eq!(map.upper[index], 10000);
                    }
                }
            }
        }
    }
}
