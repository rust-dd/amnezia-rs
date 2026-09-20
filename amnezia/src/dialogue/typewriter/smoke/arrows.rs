use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
struct Probe(Arc<AtomicUsize>);

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let visible = match label {
        "dialogue-tiffany-complete" | "dialogue-ron-144fps" => true,
        "dialogue-tiffany-arrow-hidden" | "dialogue-ron-arrow-hidden" => false,
        _ => return None,
    };
    world.init_resource::<Probe>();
    let actual = world
        .query_filtered::<&InheritedVisibility, With<view::DialogueArrow>>()
        .single(world)
        .unwrap()
        .get();
    assert_eq!(actual, visible);
    let node = world
        .query_filtered::<&Node, With<view::DialoguePanel>>()
        .single(world)
        .unwrap();
    let top = match node.top {
        Val::Px(value) => (value / 3.0) as u32,
        Val::Auto => 160,
        _ => panic!("unexpected message position"),
    };
    assert!([0, 80, 160].contains(&top));
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>("graphics/System/System.png");
    let skin = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let mut pixels = Vec::new();
    for y in 0..8 {
        for x in 0..16 {
            let background = rgba(skin, background(152 + x, 320), background(72 + y, 80));
            let frame = over(rgba(skin, 40 + x, 24 + y), background);
            let pixel = if visible {
                over(rgba(skin, 40 + x, 16 + y), frame)
            } else {
                frame
            };
            pixels.push((152 + x, top + 72 + y, pixel));
        }
    }
    Some(Snapshot {
        pixels,
        checks: world.resource::<Probe>().0.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "pause arrow ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!("dialogue pause arrow: 128 original skin/arrow pixels verified");
    }
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Probe>().0.load(Ordering::Relaxed), 4);
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn background(position: u32, length: u32) -> u32 {
    let scale = (32 << 16) / length;
    ((2 * position + 1) * scale / 2).saturating_sub(1) >> 16
}

fn over(foreground: [u8; 4], background: [u8; 4]) -> [u8; 4] {
    let alpha = u32::from(foreground[3]);
    let mut result = [0, 0, 0, 255];
    for i in 0..3 {
        result[i] = ((u32::from(foreground[i]) * alpha + u32::from(background[i]) * (255 - alpha))
            / 255) as u8;
    }
    result
}
