use crate::world::{Character, EventSprite, MainCamera, MapData};
use bevy::prelude::*;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

mod staging;

#[derive(Resource, Default)]
struct Checked(Arc<Mutex<BTreeSet<String>>>);

pub(super) struct Snapshot {
    label: String,
    pixels: Vec<(u32, u32, [u8; 4])>,
    checked: Arc<Mutex<BTreeSet<String>>>,
}

pub(super) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let (map, mut expected) = staging::cast(label)?;
    if label == "airship-sky-island" && !world.resource::<crate::state::Switches>().get(301) {
        expected.retain(|(id, _, _, _)| *id != 7);
    }
    assert_eq!(world.resource::<MapData>().map_id, map);
    assert_eq!(
        world.resource::<crate::screenfx::TintState>().tone(),
        [100.0; 4]
    );
    assert!(!world.resource::<crate::transitions::Transition>().erased());
    world.init_resource::<Checked>();
    let camera = world
        .query_filtered::<&GlobalTransform, With<MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    let node = world
        .query_filtered::<&Node, With<crate::dialogue::DialoguePanel>>()
        .single(world)
        .unwrap();
    let ui_y = match node.top {
        Val::Px(y) => (y / 3.0).round() as i32,
        _ => 160,
    };
    let actors = world
        .query::<(&EventSprite, &GlobalTransform, &InheritedVisibility)>()
        .iter(world)
        .filter(|(actor, _, visible)| visible.get() && actor.charset != "Torch")
        .map(|(actor, transform, _)| {
            let position = transform.translation() - camera;
            let size = if actor.charset.is_empty() {
                Vec2::splat(16.0)
            } else {
                Vec2::new(24.0, 32.0)
            };
            let center = Vec2::new(160.0 + position.x, 120.0 - position.y);
            (
                actor.clone(),
                Rect::from_center_size(center, size),
                position.z,
            )
        })
        .collect::<Vec<_>>();
    let hero = world
        .query::<(
            &crate::player::Player,
            &GlobalTransform,
            &InheritedVisibility,
        )>()
        .single(world)
        .unwrap();
    let hero_box = (hero.2.get() && hero.0.charset != "Torch").then(|| {
        let position = hero.1.translation() - camera;
        (
            (160.0 + position.x - 12.0).round() as i32,
            (120.0 - position.y - 16.0).round() as i32,
            position.z,
        )
    });
    let mut pixels = Vec::new();
    for (id, charset, index, tile) in expected {
        let matching = actors
            .iter()
            .filter(|(actor, _, _)| actor.id == id)
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 1, "{label}: visible original actor {id}");
        let (actor, bounds, depth) = matching[0];
        let (left, top) = (bounds.min.x.round() as i32, bounds.min.y.round() as i32);
        assert_eq!(
            (actor.charset.as_str(), actor.index, actor.tile()),
            (charset, index, tile),
            "{label}: staging actor {id}"
        );
        let handle = world
            .resource::<AssetServer>()
            .load::<Image>(crate::assets::resolve_png("CharSet", charset));
        let source = world.resource::<Assets<Image>>().get(&handle).unwrap();
        let (sx, sy) = crate::tiles::charset_source(index, actor.dir, actor.frame);
        let before = pixels.len();
        for y in 0..32 {
            for x in 0..24 {
                let (px, py) = (left + x, top + y);
                if !(0..320).contains(&px)
                    || !(0..240).contains(&py)
                    || (ui_y..ui_y + 80).contains(&py)
                {
                    continue;
                }
                if actors.iter().any(|(other, rect, oz)| {
                    other.id != id
                        && oz >= depth
                        && rect.contains(Vec2::new(px as f32 + 0.5, py as f32 + 0.5))
                }) || hero_box.is_some_and(|(hx, hy, hz)| {
                    hz >= *depth && (hx..hx + 24).contains(&px) && (hy..hy + 32).contains(&py)
                }) {
                    continue;
                }
                let color = source
                    .get_color_at(sx as u32 + x as u32, sy as u32 + y as u32)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                if color[3] == 255 {
                    pixels.push((px as u32, py as u32, color));
                }
            }
        }
        assert!(
            pixels.len() - before > 20,
            "{label}: actor {id} at ({left},{top}) lacks independent visible pixels: {}",
            pixels.len() - before
        );
    }
    Some(Snapshot {
        label: label.to_owned(),
        pixels,
        checked: world.resource::<Checked>().0.clone(),
    })
}

impl Snapshot {
    pub(super) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                "{}: cast ({x},{y}) expected {expected:?}, got {actual:?}",
                self.label
            );
        }
        self.checked.lock().unwrap().insert(self.label.clone());
        info!(
            "{}: {} original cast pixels verified",
            self.label,
            self.pixels.len()
        );
    }
}

pub(super) fn verify_finished(world: &World, labels: &[&str]) {
    let checked = world.resource::<Checked>().0.lock().unwrap();
    for &label in labels {
        assert!(checked.contains(label), "missing cast pixels: {label}");
    }
}
