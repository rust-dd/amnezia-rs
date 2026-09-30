use crate::dialogue::Dialogue;
use crate::player::{HeroHidden, Player};
use crate::state::{Switches, Variables};
use crate::world::{EventSprite, MainCamera, MapData};
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
struct Probe {
    staged: bool,
    hand_dialogue: bool,
    checks: usize,
    pixels: Arc<AtomicUsize>,
}

fn close_branch() -> bool {
    std::env::args().any(|arg| arg == "--smoke-airship-affinity")
}

pub(super) fn prepare(world: &mut World) {
    world.insert_resource(Probe::default());
    world
        .resource_mut::<Variables>()
        .set(1, if close_branch() { 8 } else { 7 });
    world.resource_mut::<Switches>().set(238, true);
}

pub(super) fn drive(world: &mut World) -> Option<&'static str> {
    if world.resource::<MapData>().map_id != 94 {
        return None;
    }
    let actors = world
        .query::<&EventSprite>()
        .iter(world)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        actors.len(),
        6,
        "the original escape map has six event entities"
    );
    for id in 1..=6 {
        assert_eq!(actors.iter().filter(|actor| actor.id == id).count(), 1);
    }
    let dialogue = world.resource::<Dialogue>();
    if dialogue.active {
        assert!(
            world.resource::<HeroHidden>().0,
            "Ron is represented by event 2 during the escape"
        );
        let visible = world
            .query_filtered::<&InheritedVisibility, With<Player>>()
            .single(world)
            .unwrap();
        assert!(
            !visible.get(),
            "the player must not duplicate the event actor"
        );
    }
    let dialogue = world.resource::<Dialogue>();
    let text = dialogue
        .boxes
        .iter()
        .flat_map(|page| &page.lines)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    if text.contains("Fogd a kezem") {
        world.resource_mut::<Probe>().hand_dialogue = true;
    }
    world.resource_mut::<Probe>().checks += 1;
    if !world.resource::<Probe>().staged
        && text.contains("Úgy látszik ugranunk kell")
        && world.resource::<Dialogue>().ready_to_advance()
        && !world.resource::<crate::teleport::Fade>().busy()
    {
        super::scenarios::verify_airship_staging(world);
        world.resource_mut::<Probe>().staged = true;
        return Some("airship-cast");
    }
    None
}

pub(super) struct Snapshot {
    samples: Vec<(u32, u32, [u8; 4])>,
    verified: Arc<AtomicUsize>,
}

pub(super) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !matches!(label, "airship-cast" | "airship-fortress-cast") {
        return None;
    }
    let fortress = label == "airship-fortress-cast";
    assert_eq!(
        world.resource::<crate::screenfx::TintState>().tone(),
        [100.0; 4]
    );
    let camera = world
        .query_filtered::<&GlobalTransform, With<MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    let actors = world
        .query::<(&EventSprite, &GlobalTransform)>()
        .iter(world)
        .filter(|(actor, _)| {
            if fortress {
                super::journey::CAST.contains(&actor.id)
            } else {
                (2..=6).contains(&actor.id)
            }
        })
        .map(|(actor, transform)| {
            let position = transform.translation() - camera;
            (
                actor.clone(),
                (160.0 + position.x - 12.0).round() as i32,
                (120.0 - position.y - 16.0).round() as i32,
            )
        })
        .collect::<Vec<_>>();
    let mut samples = Vec::new();
    for (actor, left, top) in &actors {
        let handle = world
            .resource::<AssetServer>()
            .load::<Image>(crate::assets::resolve_png("CharSet", &actor.charset));
        let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
        let (sx, sy) = crate::tiles::charset_source(actor.index, actor.dir, actor.frame);
        let before = samples.len();
        for y in 0..32 {
            for x in 0..24 {
                let (px, py) = (left + x, top + y);
                if actors.iter().any(|(other, ox, oy)| {
                    other.id != actor.id
                        && (*ox..*ox + 24).contains(&px)
                        && (*oy..*oy + 32).contains(&py)
                }) {
                    continue;
                }
                let color = image
                    .get_color_at(sx as u32 + x as u32, sy as u32 + y as u32)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                if color[3] == 255 && (0..320).contains(&px) && (80..240).contains(&py) {
                    samples.push((px as u32, py as u32, color));
                }
            }
        }
        assert!(
            samples.len() - before > 20,
            "event {} has no independently visible pixels",
            actor.id
        );
    }
    Some(Snapshot {
        samples,
        verified: if fortress {
            super::journey::pixels(world)
        } else {
            world.resource::<Probe>().pixels.clone()
        },
    })
}

impl Snapshot {
    pub(super) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.samples {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                "airship cast ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.verified.fetch_add(1, Ordering::Relaxed);
        info!(
            "airship cast: {} independent original sprite pixels verified",
            self.samples.len()
        );
    }
}

pub(super) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert!(probe.staged);
    assert_eq!(probe.hand_dialogue, close_branch());
    assert!(probe.checks > 500);
    assert_eq!(probe.pixels.load(Ordering::Relaxed), 1);
    assert!(!world.resource::<Switches>().get(238));
    assert!(world.resource::<Switches>().get(248));
    info!(
        "airship escape: {} cast updates, affinity branch={}, unique actors, staging and all five exits verified",
        probe.checks,
        close_branch()
    );
}
