use super::*;
use crate::screenfx::Fx;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

const BASES: [[u8; 3]; 3] = [[0, 0, 0], [64, 128, 192], [255; 3]];
const UI: [u8; 3] = [40, 80, 120];

mod transfers;

#[derive(Resource, Default)]
struct Checks(Arc<AtomicU32>);

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if let Some(label) = transfers::drive(world, frame) {
        return Some(label);
    }
    if frame == 600 {
        world.init_resource::<Checks>();
        let camera = world
            .query_filtered::<&Transform, With<crate::world::MainCamera>>()
            .single(world)
            .unwrap()
            .translation;
        for (index, [r, g, b]) in BASES.into_iter().enumerate() {
            world.spawn((
                Sprite::from_color(Color::srgb_u8(r, g, b), Vec2::new(32.0, 24.0)),
                Transform::from_xyz(
                    camera.x + 64.0 * (index + 1) as f32 - 160.0,
                    camera.y + 80.0,
                    200.0,
                ),
                bevy::camera::visibility::RenderLayers::layer(crate::screenfx::PICTURE_LAYER),
            ));
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
        ));
    }
    if matches!(frame, 620 | 700) {
        let params = if frame == 620 {
            vec![31, 10, 5, 31, 10, 0]
        } else {
            vec![7, 17, 31, 13, 10, 0]
        };
        world
            .resource_mut::<crate::interpreter::RunningEvent>()
            .start(
                0,
                vec![amnezia_data::EventCommand {
                    code: 11040,
                    indent: 0,
                    string: String::new(),
                    params,
                }],
            );
    }
    match frame {
        610 => Some("screen-flash-baseline"),
        630 => Some("screen-flash-strong-first"),
        650 => Some("screen-flash-strong-later"),
        690 => Some("screen-flash-strong-ended"),
        710 => Some("screen-flash-weak-first"),
        730 => Some("screen-flash-weak-later"),
        770 => Some("screen-flash-weak-ended"),
        _ => None,
    }
}

pub(crate) struct Snapshot {
    rgb: [u8; 3],
    alpha: u8,
    checked: Arc<AtomicU32>,
}

pub(crate) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    if !label.starts_with("screen-flash-") {
        return None;
    }
    let idle = label.ends_with("baseline") || label.ends_with("ended");
    let (rgb, alpha) = if idle {
        assert!(world.resource::<Fx>().flash.is_none());
        ([0; 3], 0)
    } else {
        let flash = world.resource::<Fx>().flash.as_ref().unwrap();
        assert!((1..60).contains(&flash.frames_left));
        let (rgb, mut level) = if label.contains("strong") {
            ([248, 80, 40], 31.0_f64)
        } else {
            ([56, 136, 248], 13.0_f64)
        };
        for remaining in (flash.frames_left + 1..=60).rev() {
            level -= level / f64::from(remaining);
        }
        (rgb, (level * 8.0) as u8)
    };
    Some(Snapshot {
        rgb,
        alpha,
        checked: world.resource::<Checks>().0.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for (index, base) in BASES.into_iter().enumerate() {
            let expected = std::array::from_fn::<_, 3, _>(|channel| {
                ((u32::from(base[channel]) * (255 - u32::from(self.alpha))
                    + u32::from(self.rgb[channel]) * u32::from(self.alpha))
                    / 255) as u8
            });
            for y in 30..50 {
                for x in 64 * (index as u32 + 1) - 14..64 * (index as u32 + 1) + 14 {
                    let actual = crate::display::smoke::pixel_at(image, x, y);
                    assert!(
                        actual[..3]
                            .iter()
                            .zip(expected)
                            .all(|(a, b)| a.abs_diff(b) <= 1),
                        "FlashScreen ({x},{y}): {actual:?}, expected {expected:?}"
                    );
                }
            }
        }
        for y in 30..50 {
            for x in 250..270 {
                assert_eq!(crate::display::smoke::pixel_at(image, x, y)[..3], UI);
            }
        }
        self.checked.fetch_add(1, Ordering::SeqCst);
        info!(
            "FlashScreen: 1680 original byte-blended pixels and 400 unchanged UI pixels verified"
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    transfers::verify_finished(world);
    assert_eq!(world.resource::<Checks>().0.load(Ordering::SeqCst), 7);
}
