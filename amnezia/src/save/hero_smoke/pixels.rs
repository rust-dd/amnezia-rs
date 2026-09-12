use super::*;
use crate::{assets::resolve_png, tiles, world::MainCamera};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
struct Checks(Arc<AtomicUsize>);

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checked: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let (tile, name, index) = match label {
        "saved-hero-before" => ((62, 61), "Chara1", 1),
        "saved-hero-restored" => ((62, 61), "Chara4", 3),
        "saved-hero-landed" => ((63, 61), "Chara4", 3),
        _ => return None,
    };
    world.init_resource::<Checks>();
    let camera = world
        .query_filtered::<&GlobalTransform, With<MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    let (hero, sprite, transform, visible) = world
        .query::<(&Player, &Sprite, &GlobalTransform, &InheritedVisibility)>()
        .single(world)
        .unwrap();
    assert_eq!(hero.tile(), tile);
    assert_eq!((hero.charset.as_str(), hero.index), (name, index));
    assert!(visible.get());
    assert_eq!(sprite.color.alpha(), 1.0);
    let source = world
        .resource::<AssetServer>()
        .load::<Image>(resolve_png("CharSet", name));
    assert_eq!(sprite.image, source);
    let (sx, sy) = tiles::charset_source(index, hero.dir, hero.frame);
    assert_eq!(sprite.rect, Some(Rect::new(sx, sy, sx + 24.0, sy + 32.0)));
    let position = transform.translation() - camera;
    let left = (160.0 + position.x - 12.0).round() as u32;
    let top = (120.0 - position.y - 16.0).round() as u32;
    assert!(left + 24 <= 320 && top + 32 <= 240);
    let image = world.resource::<Assets<Image>>().get(&source).unwrap();
    let mut pixels = Vec::new();
    for y in 0..32 {
        for x in 0..24 {
            let pixel = image
                .get_color_at(sx as u32 + x, sy as u32 + y)
                .unwrap()
                .to_srgba()
                .to_u8_array();
            let expected = if pixel[3] == 0 {
                [0, 0, 0, 255]
            } else {
                assert_eq!(pixel[3], 255);
                pixel
            };
            pixels.push((left + x, top + y, expected));
        }
    }
    Some(Snapshot {
        pixels,
        checked: world.resource::<Checks>().0.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "saved hero ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checked.fetch_add(1, Ordering::Relaxed);
        info!("saved hero: 768 original charset/background pixels verified");
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().0.load(Ordering::Relaxed), 3);
}
