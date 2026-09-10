use super::AnimationLibrary;
use super::cells::CellRenderer;
use super::render::{overlay_layer, overlay_translation};
use amnezia_data::{AnimationCellDef, AnimationFrameDef};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

mod timing;

const LABELS: [&str; 8] = [
    "animation-gray-23",
    "animation-gray-46",
    "animation-gray-47",
    "animation-gray-55",
    "animation-gray-72",
    "animation-scale-50",
    "animation-scale-33",
    "animation-scale-99",
];

#[derive(Resource)]
struct Fixture {
    cell: AnimationCellDef,
    image: Handle<Image>,
    entities: Vec<Entity>,
}

#[derive(Resource, Default)]
struct Checks(Vec<Arc<AtomicBool>>);

pub(crate) struct Snapshot {
    pixels: Vec<[u8; 3]>,
    complete: Arc<AtomicBool>,
    label: String,
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame >= 1180 {
        return timing::drive(world, frame);
    }
    if frame == 260 {
        world.spawn((
            Sprite::from_color(Color::BLACK, Vec2::new(320.0, 240.0)),
            Transform::from_translation(overlay_translation(Vec2::ZERO, 0.0)),
            overlay_layer(),
        ));
    }
    if (260..=1100).contains(&frame) && (frame - 260).is_multiple_of(120) {
        let index = ((frame - 260) / 120) as usize;
        if let Some(previous) = world.remove_resource::<Fixture>() {
            for entity in previous.entities {
                world.despawn(entity);
            }
        }
        let id = [23, 46, 47, 55, 72, 23, 23, 23][index];
        let mut def = world
            .resource::<AnimationLibrary>()
            .0
            .iter()
            .find(|a| a.id == id)
            .unwrap()
            .clone();
        let mut cell = def
            .frames
            .iter()
            .flat_map(|f| &f.cells)
            .find(|c| {
                c.valid
                    && c.tone_gray != 100
                    && (50..=200).contains(&c.scale)
                    && c.transparency <= 60
            })
            .unwrap()
            .clone();
        if index >= 5 {
            cell.scale = [50, 33, 99][index - 5];
            cell.tone_red = 100;
            cell.tone_green = 100;
            cell.tone_blue = 100;
            cell.tone_gray = 100;
            cell.transparency = if index == 7 { 40 } else { 0 };
        }
        cell.x = 0;
        cell.y = 0;
        let image = world
            .resource::<AssetServer>()
            .load(crate::assets::resolve_png("Battle", &def.animation_name));
        info!(
            "{}: {} cell {}, scale {}, tone {:?}, transparency {}",
            LABELS[index],
            def.animation_name,
            cell.cell_id,
            cell.scale,
            [
                cell.tone_red,
                cell.tone_green,
                cell.tone_blue,
                cell.tone_gray
            ],
            cell.transparency
        );
        def.frames = vec![AnimationFrameDef {
            cells: vec![cell.clone()],
        }];
        let entities = world
            .run_system_once(move |mut renderer: CellRenderer, mut commands: Commands| {
                renderer.spawn_frame(&mut commands, &def, 0, Vec2::ZERO)
            })
            .unwrap();
        world.insert_resource(Fixture {
            cell,
            image,
            entities,
        });
    }
    if (320..=1160).contains(&frame) && (frame - 320).is_multiple_of(120) {
        Some(LABELS[((frame - 320) / 120) as usize])
    } else {
        None
    }
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if let Some(pixels) = timing::pixels(world, label) {
        return Some(checked_snapshot(world, label, pixels));
    }
    if !LABELS.contains(&label) {
        return None;
    }
    let fixture = world.resource::<Fixture>();
    let cell = &fixture.cell;
    let source = world
        .resource::<Assets<Image>>()
        .get(&fixture.image)
        .expect("animation sheet loaded");
    let size = (96.0 * cell.scale as f64 / 100.0).floor();
    let half = (48.0 * cell.scale as f64 / 100.0).floor();
    let left = 160 - half as i32;
    let top = 120 - half as i32;
    let tone = [
        cell.tone_red,
        cell.tone_green,
        cell.tone_blue,
        cell.tone_gray,
    ]
    .map(|v| v as f32);
    let crop = tone != [100.0; 4];
    let col = cell.cell_id % 5;
    let row = cell.cell_id / 5;
    let inverse = (96.0 / size * 65536.0).trunc() / 65536.0;
    let offset_x = if crop { 0.0 } else { col as f64 * size };
    let offset_y = if crop { 0.0 } else { row as f64 * size };
    let origin_x = if crop { col * 96 } else { 0 };
    let origin_y = if crop { row * 96 } else { 0 };
    let opacity = 255 * (100 - cell.transparency.min(100)) / 100;
    let mut pixels = Vec::with_capacity(320 * 240);
    for y in 0..240 {
        for x in 0..320 {
            let dx = x - left;
            let dy = y - top;
            if dx < 0 || dy < 0 || dx >= size as i32 || dy >= size as i32 {
                pixels.push([0; 3]);
                continue;
            }
            let sx = origin_x + ((dx as f64 + offset_x + 0.5) * inverse).ceil() as u32 - 1;
            let sy = origin_y + ((dy as f64 + offset_y + 0.5) * inverse).ceil() as u32 - 1;
            let color = source
                .get_color_at(sx, sy)
                .unwrap()
                .to_srgba()
                .to_u8_array();
            let color = if color[3] == 0 {
                [0; 3]
            } else {
                crate::legacy_colors::tone::apply([color[0], color[1], color[2]], tone)
                    .map(|c| ((c as u32 * opacity + 127) / 255) as u8)
            };
            pixels.push(color);
        }
    }
    Some(checked_snapshot(world, label, pixels))
}

fn checked_snapshot(world: &mut World, label: &str, pixels: Vec<[u8; 3]>) -> Snapshot {
    let complete = Arc::new(AtomicBool::new(false));
    world.init_resource::<Checks>();
    world.resource_mut::<Checks>().0.push(complete.clone());
    Snapshot {
        pixels,
        complete,
        label: label.into(),
    }
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for (index, expected) in self.pixels.iter().enumerate() {
            let x = index as u32 % 320;
            let y = index as u32 / 320;
            let actual = image
                .get_color_at(
                    (2 * x + 1) * image.width() / 640,
                    (2 * y + 1) * image.height() / 480,
                )
                .unwrap()
                .to_srgba()
                .to_u8_array();
            assert!(
                actual[..3]
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.abs_diff(*b) <= 1),
                "{} ({x},{y}): {actual:?}, expected {expected:?}",
                self.label
            );
        }
        self.complete.store(true, Ordering::SeqCst);
        info!("{}: 76800 animation reference pixels verified", self.label);
    }
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!(checks.0.len(), LABELS.len() + timing::COUNT);
    assert!(checks.0.iter().all(|done| done.load(Ordering::SeqCst)));
}
