use super::*;
use crate::tiles::{self, LowerRender};

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 3])>,
    checked: Arc<AtomicUsize>,
}

fn color(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn background(map: &Map, image: &Image, x: u32, y: u32) -> [u8; 3] {
    let index = ((y / 16) * map.width + x / 16) as usize;
    let (dx, dy) = (x % 16, y % 16);
    assert!(!tiles::is_ab_water(map.lower[index]) && !tiles::is_block_c(map.lower[index]));
    let (sx, sy) = match tiles::lower_render(map.lower[index]) {
        LowerRender::Whole { src } => (src.0 as u32 + dx, src.1 as u32 + dy),
        LowerRender::Quarters(quarters) => {
            let q = quarters[(dy / 8 * 2 + dx / 8) as usize];
            (q.src.0 as u32 + dx % 8, q.src.1 as u32 + dy % 8)
        }
    };
    let lower = color(image, sx, sy);
    let mut rgb = if lower[3] == 0 {
        [0; 3]
    } else {
        [lower[0], lower[1], lower[2]]
    };
    if let Some((sx, sy)) = tiles::upper_source(map.upper[index]) {
        let upper = color(image, sx as u32 + dx, sy as u32 + dy);
        if upper[3] == 255 {
            rgb = [upper[0], upper[1], upper[2]];
        }
    }
    rgb
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !label.starts_with("map-character-") {
        return None;
    }
    let case = CASES[world.resource::<Probe>().case];
    let (map, page) = definition(case);
    let chip = crate::assets::load_ron::<Vec<amnezia_data::Chipset>>(&format!(
        "{}/chipsets.ron",
        crate::assets::asset_root()
    ))
    .into_iter()
    .find(|chip| chip.id == map.chipset_id)
    .unwrap();
    let camera = world
        .query_filtered::<&GlobalTransform, With<super::super::MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    let (actor, sprite, transform, visible) = world
        .query::<(
            &super::super::EventSprite,
            &Sprite,
            &GlobalTransform,
            &InheritedVisibility,
        )>()
        .iter(world)
        .find(|(actor, _, _, _)| actor.id == case.event)
        .unwrap();
    let alpha = if page.translucent { 159 } else { 255 };
    assert!((sprite.color.alpha() - alpha as f32 / 255.0).abs() < 1e-6);
    assert!(visible.get());
    let (sx, sy) = tiles::charset_source(actor.index, actor.dir, actor.frame);
    assert_eq!(sprite.rect, Some(Rect::new(sx, sy, sx + 24.0, sy + 32.0)));
    let position = transform.translation() - camera;
    let (left, top) = (
        (160.0 + position.x - 12.0).round() as i32,
        (120.0 - position.y - 16.0).round() as i32,
    );
    let actor_z = transform.translation().z;
    let occluders = world
        .query::<(
            &super::super::EventSprite,
            &Sprite,
            &GlobalTransform,
            &InheritedVisibility,
        )>()
        .iter(world)
        .filter(|(other, _, transform, visible)| {
            other.id != case.event && visible.get() && transform.translation().z >= actor_z
        })
        .map(|(_, sprite, transform, _)| {
            let position = transform.translation() - camera;
            let size = sprite.custom_size.unwrap_or(Vec2::new(24.0, 32.0));
            Rect::from_corners(
                Vec2::new(
                    160.0 + position.x - size.x / 2.0,
                    120.0 - position.y - size.y / 2.0,
                ),
                Vec2::new(
                    160.0 + position.x + size.x / 2.0,
                    120.0 - position.y + size.y / 2.0,
                ),
            )
        })
        .collect::<Vec<_>>();
    let (cx, cy) = world.resource::<super::super::MapData>().tile_center(0, 0);
    let map_left = (160.0 + cx - 8.0 - camera.x).round() as i32;
    let map_top = (120.0 - cy - 8.0 + camera.y).round() as i32;
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("CharSet", &page.graphic_name));
    let tileset = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("ChipSet", &chip.graphic));
    let images = world.resource::<Assets<Image>>();
    let source = images.get(&handle).unwrap();
    let tileset = images.get(&tileset).unwrap();
    let mut pixels = Vec::new();
    for y in 0..32 {
        for x in 0..24 {
            let rgba = color(source, sx as u32 + x, sy as u32 + y);
            let (px, py) = (left + x as i32, top + y as i32);
            let (mx, my) = (px - map_left, py - map_top);
            if occluders
                .iter()
                .any(|rect| rect.contains(Vec2::new(px as f32 + 0.5, py as f32 + 0.5)))
            {
                continue;
            }
            if rgba[3] != 255
                || !(0..320).contains(&px)
                || !(0..240).contains(&py)
                || !(0..map.width as i32 * 16).contains(&mx)
                || !(0..map.height as i32 * 16).contains(&my)
            {
                continue;
            }
            let index = (my / 16 * map.width as i32 + mx / 16) as usize;
            if page.layer != 2
                && (tiles::above_hero_lower(map.lower[index], &chip.passages_down)
                    || tiles::above_hero(map.upper[index], &chip.passages_up))
            {
                continue;
            }
            let base = background(&map, tileset, mx as u32, my as u32);
            let rgb = std::array::from_fn(|i| {
                ((u32::from(rgba[i]) * alpha + u32::from(base[i]) * (255 - alpha) + 127) / 255)
                    as u8
            });
            pixels.push((px as u32, py as u32, rgb));
        }
    }
    assert!(
        pixels.len() > 40,
        "{label}: target must have independently visible pixels"
    );
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
                "map character ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checked.fetch_add(1, Ordering::Relaxed);
        info!(
            "map character: {} original sprite/background/opacity pixels verified",
            self.pixels.len()
        );
    }
}
