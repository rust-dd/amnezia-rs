use super::*;
use amnezia_data::{EventCommand, Map};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

mod lifecycle;
mod pixels;
pub(crate) mod saved;

pub(crate) use pixels::snapshot;

#[derive(Component)]
struct Backing;

#[derive(Resource)]
struct Checks {
    camera: Vec2,
    anchor: Vec2,
    pixels: Arc<AtomicU32>,
}

const BASES: [[u8; 3]; 3] = [[0, 0, 0], [64, 128, 192], [255; 3]];
const UI: [u8; 3] = [40, 80, 120];

fn command(map_id: u32, strength: i32) -> EventCommand {
    let map = crate::assets::load_ron::<Map>(&format!(
        "{}/maps/map_{map_id:04}.ron",
        crate::assets::asset_root()
    ));
    map.events
        .iter()
        .flat_map(|event| &event.pages)
        .flat_map(|page| &page.commands)
        .find(|command| command.code == 11070 && command.params == [1, strength])
        .expect("original rain command")
        .clone()
}

pub(crate) fn start(world: &mut World, map: u32, strength: i32) {
    assert!(
        !world
            .resource::<crate::interpreter::RunningEvent>()
            .active()
    );
    world
        .resource_mut::<crate::interpreter::RunningEvent>()
        .start(0, vec![command(map, strength)]);
}

fn fixture(world: &mut World) {
    let mut rain = world.resource_mut::<Rain>();
    rain.fraction = 0.0;
    for (index, drop) in rain.drops.iter_mut().enumerate() {
        *drop = model::Drop {
            x: (index % 20 * 16) as i32,
            y: (index / 20 * 26) as i32,
            life: 12,
        };
    }
    rain.drops[0] = model::Drop {
        x: 319,
        y: 159,
        life: 12,
    };
    rain.drops[1] = model::Drop {
        x: -3,
        y: 20,
        life: 12,
    };
    rain.drops[2] = model::Drop {
        x: 40,
        y: 310,
        life: 3,
    };
}

pub(crate) fn setup(world: &mut World) {
    let camera = world
        .resource::<crate::player::CameraPan>()
        .position
        .unwrap();
    let anchor = world
        .query_filtered::<&Transform, With<MainCamera>>()
        .single(world)
        .unwrap()
        .translation
        .truncate();
    world.insert_resource(Checks {
        camera,
        anchor,
        pixels: Arc::default(),
    });
    world.insert_resource(Scroll {
        pan: Vec2::ZERO,
        previous: Some(camera),
    });
    for (index, rgb) in BASES.into_iter().enumerate() {
        let image = world.resource_mut::<Assets<Image>>().add(Image::new(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![rgb[0], rgb[1], rgb[2], 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        ));
        world.spawn((
            Sprite {
                image: image.clone(),
                custom_size: Some(Vec2::new(107.0, 10000.0)),
                ..default()
            },
            Transform::from_xyz(
                anchor.x + index as f32 * 107.0 + 53.5 - 160.0,
                anchor.y,
                90.0,
            ),
            Backing,
        ));
        if index == 0 {
            world.spawn((
                Sprite {
                    image,
                    custom_size: Some(Vec2::splat(10000.0)),
                    ..default()
                },
                Transform::from_translation(anchor.extend(80.0)),
                Backing,
            ));
        }
    }
    world.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(248.0 * 3.0),
            top: px(28.0 * 3.0),
            width: px(24.0 * 3.0),
            height: px(24.0 * 3.0),
            ..default()
        },
        BackgroundColor(Color::srgb_u8(UI[0], UI[1], UI[2])),
        GlobalZIndex(999),
        Backing,
    ));
}

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    matches!(frame, 450 | 510).then_some(KeyCode::Escape)
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        260 => {
            setup(world);
            start(world, 38, 0);
        }
        280 | 310 | 340 | 360 | 390 => fixture(world),
        300 => start(world, 28, 1),
        330 => start(world, 38, 2),
        350 => {
            world.write_message(crate::screenfx::ScreenEffect::tint(&[
                50, 100, 150, 50, 0, 0,
            ]));
        }
        370 => {
            world
                .resource_mut::<crate::player::CameraPan>()
                .command(&[2, 1, 1, 4, 0]);
        }
        380 => {
            world.write_message(crate::screenfx::ScreenEffect::shake(&[3, 5, 100, 0]));
        }
        _ => {}
    }
    match frame {
        282 => Some("weather-pixels-weak"),
        285 => Some("weather-pixels-weak-later"),
        312 => Some("weather-pixels-medium"),
        315 => Some("weather-pixels-medium-later"),
        342 => Some("weather-pixels-strong"),
        345 => Some("weather-pixels-strong-later"),
        362 => Some("weather-pixels-tone"),
        392 => Some("weather-pixels-pan-shake"),
        _ => lifecycle::drive(world, frame),
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().pixels.load(Ordering::SeqCst), 8);
    lifecycle::verify_finished(world);
}
