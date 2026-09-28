use crate::assets::{asset_root, load_ron, resolve_png};
use crate::tiles::{self, LowerRender};
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Resource, Default)]
struct Checked(Arc<AtomicBool>);

pub(super) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 3])>,
    checked: Arc<AtomicBool>,
}

pub(super) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let map_id = match label {
        "intro"
        | "map-animation-transferred"
        | "async-transition-foreground"
        | "async-transition-common"
        | "async-transition-map"
        | "async-inn-foreground"
        | "async-inn-common"
        | "async-inn-map"
        | "reserved-transfer-finished" => 3,
        "escape" => 86,
        _ => return None,
    };
    world.init_resource::<Checked>();
    assert_eq!(world.resource::<crate::world::MapData>().map_id, map_id);
    assert_eq!(
        world.resource::<crate::screenfx::TintState>().tone(),
        [100.0; 4]
    );
    assert!(!world.resource::<crate::transitions::Transition>().erased());
    assert!(!world.resource::<crate::player::HeroHidden>().0);
    let camera = world
        .query_filtered::<&GlobalTransform, With<crate::world::MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    assert_eq!(camera.truncate(), Vec2::ZERO);
    let map = load_ron::<amnezia_data::Map>(&format!("{}/maps/map_{map_id:04}.ron", asset_root()));
    assert_eq!((map.width, map.height), (20, 15));
    let chipset = load_ron::<Vec<amnezia_data::Chipset>>(&format!("{}/chipsets.ron", asset_root()))
        .into_iter()
        .find(|chipset| chipset.id == map.chipset_id)
        .unwrap();
    let server = world.resource::<AssetServer>();
    let handle = server.load::<Image>(resolve_png("ChipSet", &chipset.graphic));
    let source = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let mut pixels = Vec::new();
    for y in 176..192 {
        for x in 0..320 {
            pixels.push((x, y, map_pixel(&map, source, x, y)));
        }
    }
    if map_id == 3 {
        assert!(pixels.iter().filter(|(_, _, rgb)| *rgb != [0; 3]).count() > 1000);
    } else {
        assert!(pixels.iter().all(|(_, _, rgb)| *rgb == [0; 3]));
    }
    let (player, transform, visibility) = world
        .query::<(
            &crate::player::Player,
            &GlobalTransform,
            &InheritedVisibility,
        )>()
        .single(world)
        .unwrap();
    assert_eq!((player.charset.as_str(), player.index), ("Chara1", 0));
    assert!(visibility.get());
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(resolve_png("CharSet", "Chara1"));
    let source = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let (sx, sy) = tiles::charset_source(0, player.dir, player.frame);
    let position = transform.translation() - camera;
    let left = (160.0 + position.x - 12.0).round() as u32;
    let top = (120.0 - position.y - 16.0).round() as u32;
    let before = pixels.len();
    for y in 0..32 {
        for x in 0..24 {
            let pixel = rgba(source, sx as u32 + x, sy as u32 + y);
            if pixel[3] == 255 {
                pixels.push((left + x, top + y, [pixel[0], pixel[1], pixel[2]]));
            }
        }
    }
    assert!(pixels.len() - before > 100);
    Some(Snapshot {
        pixels,
        checked: world.resource::<Checked>().0.clone(),
    })
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn map_pixel(map: &amnezia_data::Map, image: &Image, x: u32, y: u32) -> [u8; 3] {
    let index = ((y / 16) * map.width + x / 16) as usize;
    let (dx, dy) = (x % 16, y % 16);
    let (sx, sy) = match tiles::lower_render(map.lower[index]) {
        LowerRender::Whole { src } => (src.0 as u32 + dx, src.1 as u32 + dy),
        LowerRender::Quarters(quarters) => {
            let quarter = quarters[(dy / 8 * 2 + dx / 8) as usize];
            (quarter.src.0 as u32 + dx % 8, quarter.src.1 as u32 + dy % 8)
        }
    };
    let lower = rgba(image, sx, sy);
    let mut result = [lower[0], lower[1], lower[2]];
    if lower[3] == 0 {
        result = [0; 3];
    }
    if let Some((sx, sy)) = tiles::upper_source(map.upper[index]) {
        let upper = rgba(image, sx as u32 + dx, sy as u32 + dy);
        if upper[3] != 0 {
            assert_eq!(upper[3], 255);
            result = [upper[0], upper[1], upper[2]];
        }
    }
    result
}

impl Snapshot {
    pub(super) fn verify(&self, image: &Image, label: &str) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            let matches = actual[..3]
                .iter()
                .zip(expected)
                .all(|(&a, b)| a.abs_diff(b) <= 1);
            if !matches && !cfg!(test) {
                save_failure(image, label);
            }
            assert!(
                matches,
                "{label} at ({x},{y}): expected original map/hero {expected:?}, got {actual:?}"
            );
        }
        self.checked.store(true, Ordering::Relaxed);
        info!(
            "{label}: {} original map and hero pixels verified",
            self.pixels.len()
        );
    }
}

fn save_failure(image: &Image, label: &str) {
    let path = std::env::temp_dir().join(format!(
        "amnezia-smoke-failed-{label}-{}.png",
        std::process::id()
    ));
    match image.clone().try_into_dynamic() {
        Ok(image) => match image.save(&path) {
            Ok(()) => error!("failed {label} capture saved to {}", path.display()),
            Err(error) => error!("cannot save failed {label} capture: {error}"),
        },
        Err(error) => error!("cannot convert failed {label} capture: {error}"),
    }
}

pub(super) fn verify_finished(world: &World, scenario: &str) {
    if matches!(
        scenario,
        "intro"
            | "escape"
            | "map-animations"
            | "async-transitions"
            | "async-inns"
            | "reserved-transfers"
    ) {
        assert!(world.resource::<Checked>().0.load(Ordering::Relaxed));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> Image {
        Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: 320,
                height: 240,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            &[0, 0, 0, 255],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        )
    }

    #[test]
    fn an_original_black_background_still_requires_the_hero_pixels() {
        let snapshot = Snapshot {
            pixels: vec![(0, 176, [0; 3]), (160, 120, [80, 120, 160])],
            checked: Arc::default(),
        };
        let mut image = image();
        image
            .set_color_at(160, 120, Color::srgb_u8(80, 120, 160))
            .unwrap();
        snapshot.verify(&image, "escape");
        assert!(snapshot.checked.load(Ordering::Relaxed));
    }

    #[test]
    #[should_panic(expected = "expected original map/hero")]
    fn a_missing_original_background_cannot_pass_a_logically_complete_intro() {
        Snapshot {
            pixels: vec![(48, 176, [80, 120, 160])],
            checked: Arc::default(),
        }
        .verify(&image(), "intro");
    }

    #[test]
    #[should_panic]
    fn cleared_animation_state_cannot_replace_a_verified_destination_image() {
        let mut world = World::new();
        world.init_resource::<Checked>();
        verify_finished(&world, "map-animations");
    }

    #[test]
    fn every_story_scene_requires_its_completed_pixel_verification() {
        let mut world = World::new();
        world.init_resource::<Checked>();
        world.resource::<Checked>().0.store(true, Ordering::Relaxed);
        for scenario in ["intro", "escape", "map-animations"] {
            verify_finished(&world, scenario);
        }
    }
}
