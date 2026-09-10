use super::{Effect, Picture, PictureCommand, effects::EffectState, render::PictureMaterial};
use bevy::prelude::*;

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        for (id, x, keyed) in [(1, 230.0, 0), (2, 280.0, 1)] {
            world.write_message(PictureCommand::show(
                id,
                "Cross",
                x,
                60.0,
                &[0, 0, 0, 0, 0, 800, 0, keyed, 100, 100, 100, 100, 0, 60],
            ));
        }
        world.write_message(PictureCommand::show(
            3,
            "Staff1",
            74.0,
            120.0,
            &[3, 0, 74, 120, 0, 100, 0, 0, 100, 100, 100, 100, 0, 5497976],
        ));
    }
    if frame == 400 {
        for id in 1..=3 {
            world.write_message(PictureCommand::erase(id));
        }
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/maps/map_0036.ron",
            crate::assets::asset_root()
        ));
        let fog = map
            .events
            .iter()
            .flat_map(|e| &e.pages)
            .flat_map(|p| &p.commands)
            .find(|c| c.code == 11110 && c.string == "Fog")
            .unwrap();
        world.write_message(PictureCommand::show(1, "Fog", 160.0, 120.0, &fog.params));
    }
    if frame == 500 {
        for (id, zoom, mode, strength) in [(1, 100, 0, 0), (2, 50, 2, 4)] {
            world.write_message(PictureCommand::show(
                id,
                "Fog",
                160.0,
                120.0,
                &[
                    0, 0, 0, 0, 0, zoom, 0, 0, 100, 100, 100, 100, mode, strength,
                ],
            ));
        }
    }
    if matches!(frame, 540 | 545) {
        *world.resource_mut::<bevy::time::TimeUpdateStrategy>() =
            bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::ZERO);
        let mut picture = world
            .query::<&mut Picture>()
            .iter_mut(world)
            .find(|p| p.id == 2)
            .unwrap();
        picture.effect = EffectState::show(Effect {
            mode: 2,
            strength: 4,
        });
        if frame == 545 {
            for _ in 0..16 {
                picture.effect.tick(0);
            }
        }
    }
    if frame == 550 {
        *world.resource_mut::<bevy::time::TimeUpdateStrategy>() =
            bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(
                1.0 / 60.0,
            ));
    }
    if frame == 560 {
        world.write_message(PictureCommand::show(
            2,
            "Fog",
            160.0,
            120.0,
            &[0, 0, 0, 0, 0, 120, 0, 0, 100, 100, 100, 100, 2, 1],
        ));
    }
    if frame == 570 {
        *world.resource_mut::<bevy::time::TimeUpdateStrategy>() =
            bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::ZERO);
        let mut pic = world
            .query::<&mut Picture>()
            .iter_mut(world)
            .find(|p| p.id == 2)
            .unwrap();
        pic.effect = EffectState::show(Effect {
            mode: 2,
            strength: 1,
        });
    }
    if frame == 575 {
        *world.resource_mut::<bevy::time::TimeUpdateStrategy>() =
            bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(
                1.0 / 60.0,
            ));
    }
    match frame {
        320 => Some("pictures-color-key"),
        460 => Some("pictures-fog-original"),
        544 => Some("pictures-wave-phase-0"),
        549 => Some("pictures-wave-phase-128"),
        574 => Some("pictures-wave-phase-clipped"),
        _ => None,
    }
}

pub(crate) struct PixelCheck {
    x: u32,
    y: u32,
    color: [u8; 4],
}

pub(crate) fn expected_pixels(world: &mut World, label: &str) -> Vec<PixelCheck> {
    if !label.starts_with("pictures-wave-phase-") {
        return Vec::new();
    }
    let source = world
        .query::<(&Picture, &MeshMaterial2d<PictureMaterial>)>()
        .iter(world)
        .find(|(p, _)| p.id == 2)
        .unwrap()
        .1
        .0
        .clone();
    let material = world
        .resource::<Assets<PictureMaterial>>()
        .get(&source)
        .unwrap();
    let image = world
        .resource::<Assets<Image>>()
        .get(&material.image)
        .unwrap();
    let phase = if label.ends_with("-128") {
        std::f64::consts::PI
    } else {
        0.0
    };
    let mut pixels = Vec::new();
    let clipped = label.ends_with("-clipped");
    for y in 45..195 {
        for x in 65..255 {
            let (sx, sy) = if clipped {
                let offset = (4.8 * (f64::from(y) * std::f64::consts::TAU / 38.4).sin()).trunc();
                let inverse_zoom = 54613.0 / 65536.0;
                (
                    ((f64::from(x) + 32.5 - offset) * inverse_zoom).ceil() as u32 - 1,
                    ((f64::from(y) + 25.5) * inverse_zoom).ceil() as u32 - 1,
                )
            } else {
                let row = f64::from(y) - 59.0;
                let offset = (8.0 * (phase + row * std::f64::consts::TAU / 16.0).sin()).trunc();
                let source_x = f64::from(x) - 80.0 - offset;
                if (0.0..121.0).contains(&row) && (0.0..160.0).contains(&source_x) {
                    ((source_x * 2.0) as u32, (row * 2.0) as u32)
                } else {
                    (x, y + 1)
                }
            };
            pixels.push(PixelCheck {
                x,
                y,
                color: image.get_color_at(sx, sy).unwrap().to_srgba().to_u8_array(),
            });
        }
    }
    pixels
}

pub(crate) fn verify_image(image: &Image, label: &str, expected: &[PixelCheck]) {
    let at = |x, y| crate::display::smoke::pixel_at(image, x, y);
    if !expected.is_empty() {
        let mismatches = expected
            .iter()
            .filter(|p| {
                at(p.x, p.y)[..3]
                    .iter()
                    .zip(&p.color[..3])
                    .any(|(&a, &b)| a.abs_diff(b) > 1)
            })
            .collect::<Vec<_>>();
        assert!(
            mismatches.is_empty(),
            "{label}: {} mismatched native pixels; first: {:?}",
            mismatches.len(),
            mismatches
                .first()
                .map(|p| (p.x, p.y, at(p.x, p.y), p.color))
        );
        info!(
            "{label}: {} wave pixels match the reference row formula",
            expected.len()
        );
    }
    if label != "pictures-color-key" {
        return;
    }
    let opaque = at(220, 50);
    let keyed = at(270, 50);
    for (actual, expected) in opaque[..3].iter().zip([32, 156, 0]) {
        assert!(
            actual.abs_diff(expected) <= 1,
            "opaque color key: {opaque:?}"
        );
    }
    assert_ne!(
        &keyed[..3],
        &opaque[..3],
        "keyed pixels must reveal the map"
    );
    assert_eq!(&at(10, 10)[..3], &[0, 0, 0], "opaque credits background");
    info!("picture color-key GPU assertions passed");
}
