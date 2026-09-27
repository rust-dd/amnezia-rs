use crate::interpreter::RunningEvent;
use crate::player::{CameraPan, Player};
use crate::state::Variables;
use crate::world::{Character, MapData};
use amnezia_data::EventCommand;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
struct Checks {
    states: u32,
    pixels: Arc<AtomicUsize>,
}

fn command(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        params,
        indent: 0,
        string: String::new(),
    }
}

pub(crate) fn entry() -> Vec<EventCommand> {
    vec![
        command(10810, vec![13, 60, 60]),
        command(10850, vec![2, 0, 13, 60, 60]),
        command(10840, vec![]),
    ]
}

fn relocate(world: &mut World, map: i32, x: i32, y: i32, pictures: bool) {
    assert!(!world.resource::<RunningEvent>().active());
    let mut commands = Vec::new();
    if pictures {
        for (id, fixed, x) in [(1, 0, 72), (2, 1, 248)] {
            let mut picture = command(
                11110,
                vec![id, 0, x, 72, fixed, 100, 0, 1, 100, 100, 100, 100, 0, 0],
            );
            picture.string = "Cross".into();
            commands.push(picture);
        }
    }
    commands.push(command(10850, vec![2, 0, map, x, y]));
    commands
        .extend((0..6).map(|op| command(10220, vec![0, 4500 + op, 4500 + op, 0, 6, 10001, op])));
    world.resource_mut::<RunningEvent>().start(0, commands);
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 300 {
        world.init_resource::<Checks>();
        assert_eq!(
            world.resource::<crate::vehicles::Vehicles>().save.riding,
            Some(2)
        );
        relocate(world, 4, 7, 6, true);
    }
    if frame == 400 {
        relocate(world, 13, 30, 30, false);
    }
    if frame == 460 {
        relocate(world, 13, 32, 31, false);
    }
    if (301..=500).contains(&frame) {
        let (map, x, y, screen) = if frame <= 400 {
            (4, 7, 6, (120, 112))
        } else if frame <= 460 {
            (13, 30, 30, (152, 128))
        } else {
            (13, 32, 31, (152, 128))
        };
        assert_eq!(world.resource::<MapData>().map_id, map);
        assert!(!world.resource::<super::Fade>().busy());
        assert!(!world.resource::<crate::transitions::Transition>().busy());
        let hero = world.query::<&Player>().single(world).unwrap();
        assert_eq!(hero.tile(), (x, y));
        let variables = world.resource::<Variables>();
        assert_eq!(
            (
                variables.get(4500),
                variables.get(4501),
                variables.get(4502)
            ),
            (map as i32, x, y)
        );
        assert_eq!((variables.get(4504), variables.get(4505)), screen);
        world.resource_mut::<Checks>().states += 1;
    }
    match frame {
        320 => Some("quick-transfer-interior"),
        420 => Some("quick-transfer-world"),
        480 => Some("quick-transfer-relocated"),
        _ => None,
    }
}

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checked: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !matches!(
        label,
        "quick-transfer-interior" | "quick-transfer-world" | "quick-transfer-relocated"
    ) {
        return None;
    }
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("Picture", "Cross"));
    let source = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let key = source.get_color_at(0, 0).unwrap().to_srgba().to_u8_array();
    let mut pixels = Vec::new();
    for center in [72, 248] {
        for y in 0..source.height() {
            for x in 0..source.width() {
                let color = source.get_color_at(x, y).unwrap().to_srgba().to_u8_array();
                if color[3] == 255 && color[..3] != key[..3] {
                    pixels.push((
                        center - source.width() / 2 + x,
                        72 - source.height() / 2 + y,
                        color,
                    ));
                }
            }
        }
    }
    assert!(!pixels.is_empty());
    assert!(world.resource::<CameraPan>().position.is_some());
    Some(Snapshot {
        pixels,
        checked: world.resource::<Checks>().pixels.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "quick transfer picture ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checked.fetch_add(1, Ordering::Relaxed);
        info!(
            "quick transfer: {} original picture pixels verified",
            self.pixels.len()
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!(checks.states, 200);
    assert_eq!(checks.pixels.load(Ordering::Relaxed), 3);
    info!(
        "quick transfers: 200 uninterrupted map/hero/camera/transition states and three picture captures verified"
    );
}
