use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
pub(super) struct Checks(Arc<AtomicUsize>);

pub(crate) struct Snapshot {
    checks: Arc<AtomicUsize>,
    hero: Vec<(u32, u32, [u8; 4])>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !label.starts_with("crystal-") || !label.ends_with("-continued") {
        return None;
    }
    let camera = world
        .query_filtered::<&GlobalTransform, With<crate::world::MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    let crystal_id = world.resource::<super::Probe>().crystal.id;
    let (crystal, transform) = world
        .query::<(&crate::world::EventSprite, &GlobalTransform)>()
        .iter(world)
        .find(|(actor, _)| actor.id == crystal_id)
        .unwrap();
    let (crystal_sx, crystal_sy) =
        crate::tiles::charset_source(crystal.index, crystal.dir, crystal.frame);
    let crystal_name = crystal.charset.clone();
    let crystal = transform.translation() - camera;
    let crystal_left = (160.0 + crystal.x - 12.0).round() as i32;
    let crystal_top = (120.0 - crystal.y - 16.0).round() as i32;
    let (hero, transform) = world
        .query::<(&crate::player::Player, &GlobalTransform)>()
        .single(world)
        .unwrap();
    let position = transform.translation() - camera;
    let (left, top) = (
        (160.0 + position.x - 12.0).round() as i32,
        (120.0 - position.y - 16.0).round() as i32,
    );
    let (sx, sy) = crate::tiles::charset_source(hero.index, hero.dir, hero.frame);
    let data = world.resource::<crate::world::MapData>();
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_{:04}.ron",
        crate::assets::asset_root(),
        data.map_id
    ));
    let chip = crate::assets::load_ron::<Vec<amnezia_data::Chipset>>(&format!(
        "{}/chipsets.ron",
        crate::assets::asset_root()
    ))
    .into_iter()
    .find(|chip| chip.id == map.chipset_id)
    .unwrap();
    let (cx, cy) = data.tile_center(0, 0);
    let map_left = (160.0 + cx - 8.0 - camera.x).round() as i32;
    let map_top = (120.0 - cy - 8.0 + camera.y).round() as i32;
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("CharSet", &hero.charset));
    let tileset = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("ChipSet", &chip.graphic));
    let crystal_image = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("CharSet", &crystal_name));
    let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let tileset = world.resource::<Assets<Image>>().get(&tileset).unwrap();
    let crystal_image = world
        .resource::<Assets<Image>>()
        .get(&crystal_image)
        .unwrap();
    let mut pixels = Vec::new();
    for y in 0..32 {
        for x in 0..24 {
            let pixel = image
                .get_color_at(sx as u32 + x, sy as u32 + y)
                .unwrap()
                .to_srgba()
                .to_u8_array();
            let (x, y) = (left + x as i32, top + y as i32);
            let behind_crystal = (crystal_left..crystal_left + 24).contains(&x)
                && (crystal_top..crystal_top + 32).contains(&y)
                && crystal_image
                    .get_color_at(
                        crystal_sx as u32 + (x - crystal_left) as u32,
                        crystal_sy as u32 + (y - crystal_top) as u32,
                    )
                    .unwrap()
                    .alpha()
                    > 0.0;
            let (mx, my) = ((x - map_left).div_euclid(16), (y - map_top).div_euclid(16));
            if !data.contains_tile(mx, my) {
                continue;
            }
            if overhead(
                &map,
                &chip,
                tileset,
                (x - map_left) as u32,
                (y - map_top) as u32,
            ) {
                continue;
            }
            if pixel[3] == 255 && (0..320).contains(&x) && (0..240).contains(&y) && !behind_crystal
            {
                pixels.push((x as u32, y as u32, pixel));
            }
        }
    }
    assert!(pixels.len() > 30);
    if map.events.iter().any(|event| {
        event.pages.iter().any(|page| {
            page.commands
                .iter()
                .any(|command| command.code == 11110 && command.string == "Fog")
        })
    }) {
        crate::picture::smoke::checkpoint::apply_original_ship_fog(world, &mut pixels);
    }
    Some(Snapshot {
        checks: world.resource::<Checks>().0.clone(),
        hero: pixels,
    })
}

fn overhead(
    map: &amnezia_data::Map,
    chip: &amnezia_data::Chipset,
    image: &Image,
    x: u32,
    y: u32,
) -> bool {
    use crate::tiles::{self, LowerRender};
    let index = (y / 16 * map.width + x / 16) as usize;
    let (dx, dy) = (x % 16, y % 16);
    if tiles::above_hero_lower(map.lower[index], &chip.passages_down) {
        let (sx, sy) = match tiles::lower_render(map.lower[index]) {
            LowerRender::Whole { src } => (src.0 as u32 + dx, src.1 as u32 + dy),
            LowerRender::Quarters(quarters) => {
                let q = quarters[(dy / 8 * 2 + dx / 8) as usize];
                (q.src.0 as u32 + dx % 8, q.src.1 as u32 + dy % 8)
            }
        };
        if image.get_color_at(sx, sy).unwrap().alpha() > 0.0 {
            return true;
        }
    }
    if tiles::above_hero(map.upper[index], &chip.passages_up)
        && let Some((sx, sy)) = tiles::upper_source(map.upper[index])
    {
        return image
            .get_color_at(sx as u32 + dx, sy as u32 + dy)
            .unwrap()
            .alpha()
            > 0.0;
    }
    false
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.hero {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                "crystal continuation ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "crystal continuation: {} original hero pixels verified",
            self.hero.len()
        );
    }
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().0.load(Ordering::Relaxed), 1);
}
