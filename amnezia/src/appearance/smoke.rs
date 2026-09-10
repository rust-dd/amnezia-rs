use super::{Appearance, Player, SpriteChange};
use crate::gamedata::GameData;
use crate::state::Party;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const LABELS: [&str; 10] = [
    "actor-graphic-1",
    "actor-graphic-2",
    "actor-graphic-3",
    "actor-graphic-4",
    "actor-graphic-5",
    "actor-graphic-6",
    "actor-graphic-7",
    "actor-graphic-8",
    "actor-graphic-9",
    "actor-graphic-10",
];

#[derive(Resource, Default)]
struct Verified(Arc<AtomicUsize>);

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        world.init_resource::<Verified>();
        world.insert_resource(ClearColor(Color::BLACK));
        world
            .resource_mut::<crate::screenfx::TintState>()
            .set_tone([100.0; 4]);
        let scene = world
            .query_filtered::<Entity, With<crate::world::MapScene>>()
            .iter(world)
            .collect::<Vec<_>>();
        for entity in scene {
            world.despawn(entity);
        }
    }
    if (300..=840).contains(&frame) && (frame - 300).is_multiple_of(60) {
        world
            .resource_mut::<Party>()
            .restore(vec![1 + (frame - 300) / 60]);
    }
    if frame == 940 {
        world.resource_mut::<Party>().restore(vec![2, 1]);
        world.write_message(SpriteChange {
            actor_id: 1,
            charset: "Poses2".into(),
            index: 4,
        });
    }
    if matches!(frame, 1000 | 1080 | 1160) {
        world
            .resource_mut::<Party>()
            .restore(if frame == 1080 { vec![] } else { vec![1, 2] });
    }
    if (340..=880).contains(&frame) && (frame - 340).is_multiple_of(60) {
        return Some(LABELS[(frame - 340) as usize / 60]);
    }
    match frame {
        980 => Some("actor-graphic-inactive-override"),
        1040 => Some("actor-graphic-restored-override"),
        1120 => Some("actor-graphic-empty"),
        1200 => Some("actor-graphic-returned-override"),
        _ => None,
    }
}

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    verified: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let id = LABELS
        .iter()
        .position(|&l| l == label)
        .map(|i| i as u32 + 1);
    let (graphic, index) = if let Some(id) = id {
        let actor = world.resource::<GameData>().actor(id).unwrap();
        (actor.character_name.clone(), actor.character_index)
    } else {
        match label {
            "actor-graphic-inactive-override" => ("Chara1".into(), 1),
            "actor-graphic-restored-override" | "actor-graphic-returned-override" => {
                ("Poses2".into(), 4)
            }
            "actor-graphic-empty" => (String::new(), 0),
            _ => return None,
        }
    };
    let camera = world
        .query_filtered::<&GlobalTransform, With<crate::world::MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    let (player, sprite, transform, visible) = world
        .query::<(&Player, &Sprite, &GlobalTransform, &InheritedVisibility)>()
        .single(world)
        .unwrap();
    assert_eq!(
        (&player.charset, player.index),
        (&graphic, index),
        "{label}"
    );
    assert_eq!(visible.get(), !graphic.is_empty(), "{label}");
    let pixels = if graphic.is_empty() {
        (0..320 * 240)
            .map(|i| (i % 320, i / 320, [0, 0, 0, 255]))
            .collect::<Vec<_>>()
    } else {
        let source = world
            .resource::<AssetServer>()
            .load::<Image>(crate::assets::resolve_png("CharSet", &graphic));
        assert_eq!(sprite.image, source, "{label}");
        assert_eq!(sprite.color.alpha(), 1.0);
        let (sx, sy) = crate::tiles::charset_source(index, player.dir, player.frame);
        assert_eq!(sprite.rect, Some(Rect::new(sx, sy, sx + 24.0, sy + 32.0)));
        let position = transform.translation() - camera;
        let left = (160.0 + position.x - 12.0).round() as u32;
        let top = (120.0 - position.y - 16.0).round() as u32;
        let image = world.resource::<Assets<Image>>().get(&source).unwrap();
        let mut pixels = Vec::new();
        for y in 0..32 {
            for x in 0..24 {
                let pixel = image
                    .get_color_at(sx as u32 + x, sy as u32 + y)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                if pixel[3] == 255 {
                    pixels.push((left + x, top + y, pixel));
                }
            }
        }
        assert!(pixels.len() > 50, "{label}");
        pixels
    };
    Some(Snapshot {
        pixels,
        verified: world.resource::<Verified>().0.clone(),
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
                    .all(|(&a, e)| a.abs_diff(e) <= 1),
                "actor graphic ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.verified.fetch_add(1, Ordering::Relaxed);
        info!(
            "actor graphic: {} source-asset GPU pixels verified",
            self.pixels.len()
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Verified>().0.load(Ordering::Relaxed), 14);
    assert_eq!(world.resource::<Appearance>().get(1), Some(("Poses2", 4)));
}
