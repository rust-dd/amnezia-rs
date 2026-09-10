use super::{Canvas, NATIVE_SIZE, output_scale};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::view::screenshot::{ScreenshotCaptured, save_to_disk};
use bevy::window::PrimaryWindow;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Pair {
    native: Option<Image>,
    output: Option<Image>,
    complete: bool,
}

#[derive(Clone)]
pub(crate) struct Snapshot(Arc<Mutex<Pair>>, String);

#[derive(Resource, Default)]
struct Checks(Vec<Snapshot>);

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        world
            .resource_mut::<crate::interpreter::RunningEvent>()
            .start(
                0,
                vec![amnezia_data::EventCommand {
                    code: 10110,
                    indent: 0,
                    string: "Árvíztűrő tükörfúrógép.\n320×240".into(),
                    params: vec![],
                }],
            );
    }
    let size = match frame {
        360 => Some((1280.0, 720.0)),
        480 => Some((720.0, 960.0)),
        600 => Some((1001.0, 751.0)),
        720 => Some((160.0, 120.0)),
        840 => Some((960.0, 720.0)),
        _ => None,
    };
    if let Some((w, h)) = size {
        world
            .query_filtered::<&mut Window, With<PrimaryWindow>>()
            .single_mut(world)
            .unwrap()
            .resolution
            .set(w, h);
    }
    match frame {
        320 => Some("display-normal"),
        380 => Some("display-wide"),
        500 => Some("display-portrait"),
        620 => Some("display-odd-size"),
        740 => Some("display-small"),
        860 => Some("display-restored"),
        _ => None,
    }
}

pub(crate) fn capture_native(world: &mut World, label: &str, prefix: &str) -> Option<Snapshot> {
    if !matches!(
        label,
        "display-normal"
            | "display-wide"
            | "display-portrait"
            | "display-odd-size"
            | "display-small"
            | "display-restored"
    ) {
        return None;
    }
    let snapshot = Snapshot(Arc::default(), label.into());
    let native = snapshot.clone();
    let canvas = world.resource::<Canvas>().0.clone();
    let template = world
        .resource::<Assets<Image>>()
        .get(&canvas)
        .unwrap()
        .clone();
    world
        .spawn(Readback::texture(canvas))
        .observe(
            move |event: On<ReadbackComplete>, mut commands: Commands, mut done: Local<bool>| {
                if *done {
                    return;
                }
                *done = true;
                assert_eq!(
                    event.data.len(),
                    (NATIVE_SIZE.x * NATIVE_SIZE.y * 4) as usize
                );
                let mut image = template.clone();
                image.data = Some(event.data.clone());
                commands.trigger(ScreenshotCaptured {
                    entity: event.entity,
                    image,
                });
                commands.entity(event.entity).despawn();
            },
        )
        .observe(save_to_disk(
            std::env::temp_dir().join(format!("{prefix}-{label}-native.png")),
        ))
        .observe(move |captured: On<ScreenshotCaptured>| native.submit(&captured.image, true));
    world.init_resource::<Checks>();
    world.resource_mut::<Checks>().0.push(snapshot.clone());
    Some(snapshot)
}

impl Snapshot {
    pub(crate) fn submit(&self, image: &Image, native: bool) {
        let mut pair = self.0.lock().unwrap();
        if native {
            pair.native = Some(image.clone());
        } else {
            pair.output = Some(image.clone());
        }
        if let (Some(native), Some(output)) = (&pair.native, &pair.output) {
            verify_pair(native, output, &self.1);
            pair.native = None;
            pair.output = None;
            pair.complete = true;
        }
    }
}

fn pixel(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn verify_pair(native: &Image, output: &Image, label: &str) {
    assert_eq!(native.size(), NATIVE_SIZE);
    let size = output.size();
    let scale = output_scale(size);
    let drawn = NATIVE_SIZE.as_vec2() * scale;
    let offset = ((size.as_vec2() - drawn) / 2.0).floor();
    let mut visible = 0;
    for y in 0..size.y {
        for x in 0..size.x {
            let color = pixel(output, x, y);
            let position = Vec2::new(x as f32, y as f32) - offset;
            if position.x < 0.0
                || position.y < 0.0
                || position.x >= drawn.x
                || position.y >= drawn.y
            {
                assert_eq!(
                    &color[..3],
                    &[0, 0, 0],
                    "{label}: nonblack letterbox at ({x}, {y})"
                );
            } else if scale >= 1.0 {
                let source = (position / scale).floor().as_uvec2();
                let expected = pixel(native, source.x, source.y);
                assert!(
                    color[..3]
                        .iter()
                        .zip(&expected[..3])
                        .all(|(a, b)| a.abs_diff(*b) <= 1),
                    "{label}: ({x}, {y}) does not match native {source}: {color:?} vs {expected:?}"
                );
            }
            visible += usize::from(color[..3].iter().any(|&v| v > 16));
        }
    }
    assert!(
        visible > (size.x * size.y) as usize / 100,
        "{label}: empty output"
    );
    info!("{label}: native canvas, letterbox and {scale}x presentation verified");
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!(checks.0.len(), 6);
    for check in &checks.0 {
        assert!(
            check.0.lock().unwrap().complete,
            "{} capture did not finish",
            check.1
        );
    }
}
