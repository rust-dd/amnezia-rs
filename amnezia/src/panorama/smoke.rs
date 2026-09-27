use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

mod phase;
pub(crate) use phase::drive;

#[derive(Resource, Default)]
struct Checked(Arc<AtomicU32>);

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 3])>,
    verified: Arc<AtomicU32>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if label != "panorama" && !label.starts_with("panorama-phase-") {
        return None;
    }
    world.init_resource::<Checked>();
    let map = world.resource::<MapData>();
    assert_eq!(map.map_id, 94);
    let camera = world
        .resource::<crate::player::CameraPan>()
        .position
        .unwrap();
    let rendered = world
        .query_filtered::<&Transform, With<MainCamera>>()
        .single(world)
        .unwrap()
        .translation
        .truncate();
    let shake = rendered - camera;
    assert_eq!(shake.y, 0.0);
    assert_eq!(shake.x, shake.x.trunc());
    if label == "panorama-phase-shake" {
        assert!(shake.x.abs() >= 1.0);
    } else {
        assert_eq!(shake, Vec2::ZERO);
    }
    let panorama = world.resource::<Panorama>();
    assert_eq!(panorama.definition.as_ref().unwrap().name, "Sky");
    let phase = world.resource::<phase::Reference>().phase;
    assert_eq!(panorama.motion.as_ref().unwrap().phase, phase);
    let images = world.resource::<Assets<Image>>();
    let source = images
        .get(&world.resource::<BackgroundImage>().0.as_ref().unwrap().1)
        .unwrap();
    assert_eq!(source.size(), UVec2::new(640, 480));
    let start = [
        (-((-phase[0] / 16) / 2) + shake.x as i64).rem_euclid(640),
        (-((-phase[1] / 16) / 2)).rem_euclid(480),
    ];
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0094.ron",
        crate::assets::asset_root()
    ));
    let tileset = world
        .resource::<AssetServer>()
        .load::<Image>(resolve_png("ChipSet", "Draco2"));
    let tileset = images.get(&tileset).unwrap();
    let tone = world.resource::<crate::screenfx::TintState>().tone();
    let mut pixels = Vec::new();
    for y in 208..240 {
        for x in 0..320 {
            let index = ((y / 16) * 20 + x / 16) as usize;
            assert_eq!(map.lower[index], 5037);
            assert_eq!(map.upper[index], 10000);
            assert_eq!(
                tileset
                    .get_color_at(208 + x % 16, 96 + y % 16)
                    .unwrap()
                    .alpha(),
                0.0
            );
            let rgba = source
                .get_color_at((x + start[0] as u32) % 640, (y + start[1] as u32) % 480)
                .unwrap()
                .to_srgba()
                .to_u8_array();
            let rgb = crate::legacy_colors::tone::apply([rgba[0], rgba[1], rgba[2]], tone);
            pixels.push((x, y, rgb));
        }
    }
    Some(Snapshot {
        pixels,
        verified: world.resource::<Checked>().0.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        assert_eq!(self.pixels.len(), 10240);
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual[..3]
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                "original Sky at ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.verified.fetch_add(1, Ordering::SeqCst);
        info!("panorama: 10240 original Sky pixels verified behind transparent airship tiles");
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checked>().0.load(Ordering::SeqCst), 5);
    phase::verify_finished(world);
}
