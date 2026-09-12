use super::*;
use crate::world::{MapData, MoveQueue};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
struct Checks(Arc<AtomicUsize>);

pub(crate) fn configure(app: &mut App) {
    app.init_resource::<Checks>();
}

pub(crate) fn verify_target(world: &mut World) {
    let state = super::snapshot(world).cast.unwrap();
    let library = world.resource::<AnimationLibrary>();
    let def = library.0.iter().find(|def| def.id == state.id).unwrap();
    let timing = def
        .timings
        .iter()
        .filter(|timing| {
            timing.flash_scope == 1 && timing.frame > 0 && (timing.frame - 1) * 2 < state.elapsed
        })
        .max_by_key(|timing| timing.frame)
        .unwrap();
    let age = state.elapsed - 1 - (timing.frame - 1) * 2;
    assert!(age <= 10);
    let level = ((7 - age.div_ceil(2)) * timing.flash_power / 6).min(31);
    let expected = [
        timing.flash_red * 8,
        timing.flash_green * 8,
        timing.flash_blue * 8,
        level * 8,
    ]
    .map(|value| value as u8);
    let flash = world
        .query_filtered::<&crate::legacy_colors::flash::SpriteFlash, With<Player>>()
        .single(world)
        .unwrap();
    assert_eq!(flash.0, expected);
}

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    complete: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !matches!(label, "saved-animation-before" | "saved-animation-restored") {
        return None;
    }
    verify_target(world);
    let camera = world
        .query_filtered::<&Transform, With<MainCamera>>()
        .single(world)
        .unwrap()
        .translation;
    let mut heroes = world.query::<(&Player, &MoveQueue)>();
    let (hero, queue) = heroes.single(world).unwrap();
    let ground = queue.ground_position(hero, world.resource::<MapData>());
    let state = super::snapshot(world).cast.unwrap();
    assert_eq!(
        (state.id, state.target, state.global, state.elapsed / 2),
        (62, AnimTarget::Hero, false, 10)
    );
    let def = world
        .resource::<AnimationLibrary>()
        .0
        .iter()
        .find(|def| def.id == 62)
        .unwrap();
    let cells = def.frames[10]
        .cells
        .iter()
        .filter(|cell| cell.valid)
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 1);
    let cell = cells[0];
    assert_eq!(
        (cell.scale, cell.transparency, cell.x, cell.y),
        (100, 0, 0, 0)
    );
    let source = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("Battle", &def.animation_name));
    let image = world.resource::<Assets<Image>>().get(&source).unwrap();
    let left = 160 + ground.x.floor() as i32 - camera.x.floor() as i32 - 48;
    let top = 120 + camera.y.ceil() as i32 - ground.y.ceil() as i32 - 4 - 48;
    let mut pixels = Vec::new();
    for y in 0..96 {
        for x in 0..96 {
            let color = image
                .get_color_at(cell.cell_id % 5 * 96 + x, cell.cell_id / 5 * 96 + y)
                .unwrap()
                .to_srgba()
                .to_u8_array();
            let (x, y) = (left + x as i32, top + y as i32);
            if color[3] == 255 && (0..320).contains(&x) && (0..240).contains(&y) {
                pixels.push((x as u32, y as u32, color));
            }
        }
    }
    assert!(pixels.len() > 300);
    Some(Snapshot {
        pixels,
        complete: world.resource::<Checks>().0.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                "saved animation ({x},{y}): {actual:?}, expected {expected:?}"
            );
        }
        self.complete.fetch_add(1, Ordering::SeqCst);
        info!(
            "saved map animation: {} original bitmap pixels verified",
            self.pixels.len()
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().0.load(Ordering::SeqCst), 2);
}
