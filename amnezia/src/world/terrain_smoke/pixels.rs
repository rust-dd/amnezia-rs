use super::*;
use crate::tiles::{self, LowerRender};

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 3])>,
    checked: Arc<AtomicUsize>,
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn background(map: &Map, chip: &Image, x: i32, y: i32) -> [u8; 3] {
    let x = x.rem_euclid(map.width as i32 * 16) as u32;
    let y = y.rem_euclid(map.height as i32 * 16) as u32;
    let index = (y / 16 * map.width + x / 16) as usize;
    let (dx, dy) = (x % 16, y % 16);
    assert!(!tiles::is_ab_water(map.lower[index]) && !tiles::is_block_c(map.lower[index]));
    let (sx, sy) = match tiles::lower_render(map.lower[index]) {
        LowerRender::Whole { src } => (src.0 as u32 + dx, src.1 as u32 + dy),
        LowerRender::Quarters(parts) => {
            let part = parts[(dy / 8 * 2 + dx / 8) as usize];
            (part.src.0 as u32 + dx % 8, part.src.1 as u32 + dy % 8)
        }
    };
    let lower = rgba(chip, sx, sy);
    let mut result = if lower[3] == 0 {
        [0; 3]
    } else {
        [lower[0], lower[1], lower[2]]
    };
    if let Some((sx, sy)) = tiles::upper_source(map.upper[index]) {
        let upper = rgba(chip, sx as u32 + dx, sy as u32 + dy);
        if upper[3] == 255 {
            result = [upper[0], upper[1], upper[2]];
        }
    }
    result
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !label.starts_with("terrain-") || !(label.ends_with("-ground") || label.ends_with("-flight"))
    {
        return None;
    }
    let flying = label.ends_with("-flight");
    let case = CASES[world.resource::<Probe>().case];
    let (map, chip, position) = definition(case);
    let camera = world
        .query_filtered::<&GlobalTransform, With<super::super::MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    let (charset, index, dir, frame, transform) = if flying {
        let state = world.resource::<Vehicles>().save.vehicles[2].clone();
        let (sprite, transform, visible) = world
            .query::<(
                &crate::vehicles::VehicleSprite,
                &GlobalTransform,
                &InheritedVisibility,
            )>()
            .iter(world)
            .find(|(sprite, _, _)| sprite.0 == 2)
            .unwrap();
        assert_eq!(sprite.0, 2);
        assert!(visible.get());
        let transform = transform.translation();
        let hero_visible = world
            .query_filtered::<&InheritedVisibility, With<Player>>()
            .single(world)
            .unwrap();
        assert!(!hero_visible.get());
        (
            state.definition.charset,
            state.definition.index,
            state.dir,
            state.frame,
            transform,
        )
    } else {
        let (hero, transform, visible) = world
            .query::<(&Player, &GlobalTransform, &InheritedVisibility)>()
            .single(world)
            .unwrap();
        assert!(visible.get());
        (
            hero.charset.clone(),
            hero.index,
            hero.dir,
            hero.frame,
            transform.translation(),
        )
    };
    let (sx, sy) = tiles::charset_source(index, dir, frame);
    let point = transform - camera;
    let left = (160.0 + point.x - 12.0).round() as i32;
    let top = (120.0 - point.y - 16.0).round() as i32;
    let (cx, cy) = world
        .resource::<MapData>()
        .tile_center(position.0, position.1);
    if flying {
        assert_eq!(transform.y, cy + tiles::CHAR_Y_OFFSET + 16.0);
    }
    let map_left = (160.0 + cx - camera.x - position.0 as f32 * 16.0 - 8.0).round() as i32;
    let map_top = (120.0 - cy + camera.y - position.1 as f32 * 16.0 - 8.0).round() as i32;
    let server = world.resource::<AssetServer>();
    let actor_handle = server.load::<Image>(crate::assets::resolve_png("CharSet", &charset));
    let chip_handle = server.load::<Image>(crate::assets::resolve_png("ChipSet", &chip.graphic));
    let images = world.resource::<Assets<Image>>();
    let actor = images.get(&actor_handle).unwrap();
    let tileset = images.get(&chip_handle).unwrap();
    let mut pixels = Vec::new();
    let mut translucent = 0;
    for y in 0..32 {
        for x in 0..24 {
            let source = rgba(actor, sx as u32 + x, sy as u32 + y);
            let (px, py) = (left + x as i32, top + y as i32);
            if source[3] != 255 || !(0..320).contains(&px) || !(0..240).contains(&py) {
                continue;
            }
            let rgb = if !flying && case.terrain == 2 && y >= 22 {
                let base = background(&map, tileset, px - map_left, py - map_top);
                translucent += 1;
                std::array::from_fn(|i| {
                    ((u32::from(source[i]) * 128 + u32::from(base[i]) * 127 + 127) / 255) as u8
                })
            } else {
                [source[0], source[1], source[2]]
            };
            pixels.push((px as u32, py as u32, rgb));
        }
    }
    assert!(
        pixels.len() > 80,
        "{label}: original character must be visible"
    );
    if !flying && case.terrain == 2 {
        assert!(
            translucent > 20,
            "{label}: forest must exercise the lower ten rows"
        );
    }
    edges::append(case.map, &map, tileset, (map_left, map_top), &mut pixels);
    Some(Snapshot {
        pixels,
        checked: world.resource::<Probe>().checked.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual[..3]
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                "original terrain pixel ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checked.fetch_add(1, Ordering::Relaxed);
        info!(
            "original terrain: {} sprite/background/bush pixels verified",
            self.pixels.len()
        );
    }
}
