use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

#[derive(Resource, Default)]
struct Checks(Arc<AtomicU32>);

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1030 {
        world.init_resource::<Checks>();
        world.write_message(PictureCommand::show(
            1,
            "Fog",
            160.0,
            120.0,
            &[0, 0, 0, 0, 0, 100, 0, 0, 100, 100, 100, 100, 1, 64],
        ));
    }
    if matches!(frame, 1040 | 1080 | 1120) {
        *world.resource_mut::<bevy::time::TimeUpdateStrategy>() =
            bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::ZERO);
        let mut picture = world.query::<&mut Picture>().single_mut(world).unwrap();
        picture.effect = EffectState::show(Effect {
            mode: 1,
            strength: 64,
        });
        for _ in 0..1 + (frame - 1040) / 40 {
            picture.effect.tick(0);
        }
    }
    if matches!(frame, 1045 | 1085 | 1125) {
        *world.resource_mut::<bevy::time::TimeUpdateStrategy>() =
            bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(
                1.0 / 60.0,
            ));
    }
    if frame == 1130 {
        world.write_message(PictureCommand::erase(1));
    }
    match frame {
        1044 => Some("pictures-rotation-quarter"),
        1084 => Some("pictures-rotation-half"),
        1124 => Some("pictures-rotation-three-quarters"),
        _ => None,
    }
}

pub(crate) struct Snapshot {
    pixels: Vec<super::PixelCheck>,
    checked: Arc<AtomicU32>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let quarter = match label {
        "pictures-rotation-quarter" => 1,
        "pictures-rotation-half" => 2,
        "pictures-rotation-three-quarters" => 3,
        _ => return None,
    };
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("Picture", "Fog"));
    let source = world.resource::<Assets<Image>>().get(&handle).unwrap();
    assert_eq!(source.size(), UVec2::new(320, 242));
    let mut pixels = Vec::new();
    for y in 20..220 {
        for x in 60..260 {
            let (sx, sy) = match quarter {
                1 => (y + 40, 280 - x),
                2 => (319 - x, 240 - y),
                3 => (279 - y, x - 39),
                _ => unreachable!(),
            };
            let mut color = source
                .get_color_at(sx, sy)
                .unwrap()
                .to_srgba()
                .to_u8_array();
            color[3] = 255;
            pixels.push(super::PixelCheck { x, y, color });
        }
    }
    Some(Snapshot {
        pixels,
        checked: world.resource::<Checks>().0.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for pixel in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, pixel.x, pixel.y);
            assert!(
                actual
                    .iter()
                    .zip(pixel.color)
                    .all(|(a, b)| a.abs_diff(b) <= 1),
                "rotated picture ({},{}): expected {:?}, got {actual:?}",
                pixel.x,
                pixel.y,
                pixel.color,
            );
        }
        self.checked.fetch_add(1, Ordering::Relaxed);
        info!("picture rotation: 40000 original image pixels verified");
    }
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().0.load(Ordering::Relaxed), 3);
}
