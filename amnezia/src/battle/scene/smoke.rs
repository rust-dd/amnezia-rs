use super::*;
use crate::legacy_colors::{flash::SpriteFlash, hue::HueShift, tone::SpriteTone};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    shake_checks: Option<Arc<AtomicUsize>>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !matches!(
        label,
        "battle-target"
            | "battle-target-flash"
            | "battle-target-fade"
            | "battle-target-clear"
            | "battle-hit-visible"
            | "battle-hit-hidden"
            | "battle-hit-restored"
            | "battle-action-flash"
            | "battle-action-fade"
            | "battle-tone"
            | "battle-tone-light"
            | "battle-shake-right"
            | "battle-shake-left"
            | "battle-shake-restored"
    ) {
        return None;
    }
    let mut enemies = world
        .query::<(&Battler, &Sprite, &HueShift, &SpriteFlash, &SpriteTone)>()
        .iter(world)
        .map(|(b, s, h, f, t)| {
            (
                b.index,
                b.base,
                h.original.clone(),
                h.degrees,
                f.0,
                s.color.alpha(),
                t.0,
            )
        })
        .collect::<Vec<_>>();
    enemies.sort_by_key(|enemy| enemy.0);
    let expected_flash = match label {
        "battle-target-flash" => 192,
        "battle-target-fade" => 96,
        "battle-action-flash" => 80,
        "battle-action-fade" => 64,
        "battle-shake-right" => 72,
        "battle-shake-left" => 40,
        _ => 0,
    };
    assert_eq!(enemies[0].4[3], expected_flash, "{label}");
    assert_eq!(
        enemies[0].5,
        if label == "battle-hit-hidden" {
            0.0
        } else {
            1.0
        }
    );
    let (backdrop, tone) = world
        .query_filtered::<(&HueShift, &SpriteTone), (With<SceneEntity>, Without<Battler>)>()
        .single(world)
        .map(|(hue, tone)| (hue.original.clone(), tone.0))
        .unwrap();
    assert_eq!(
        tone,
        match label {
            "battle-tone" => [50.0, 100.0, 150.0, 0.0],
            "battle-tone-light" => [200.0, 200.0, 200.0, 100.0],
            _ => [100.0; 4],
        }
    );
    let images = world.resource::<Assets<Image>>();
    let backdrop = images.get(&backdrop).unwrap();
    assert_eq!(backdrop.size(), UVec2::new(320, 160));
    let shift = match label {
        "battle-shake-right" => 5,
        "battle-shake-left" => -6,
        _ => 0,
    };
    let mut expected = (0..320 * 160)
        .map(|i| {
            let x = (i % 320) as i32 - shift;
            if !(0..320).contains(&x) {
                return [0, 0, 0, 255];
            }
            let pixel = rgba(backdrop, x as u32, i / 320);
            let rgb = crate::legacy_colors::tone::apply([pixel[0], pixel[1], pixel[2]], tone);
            [rgb[0], rgb[1], rgb[2], pixel[3]]
        })
        .collect::<Vec<_>>();
    let mut sampled = vec![false; expected.len()];
    for y in 32..156 {
        for x in 0..320 {
            sampled[y * 320 + x] = true;
        }
    }
    for (_, base, source, hue, flash, opacity, tone) in enemies {
        let source = images.get(&source).unwrap();
        let left = base.x as i32 + shift + 160 - source.width() as i32 / 2;
        let top = base.y as i32 + 120 - source.height() as i32 / 2;
        for y in 0..source.height() {
            for x in 0..source.width() {
                let dx = left + x as i32;
                let dy = top + y as i32;
                let pixel = rgba(source, x, y);
                if pixel[3] != 255 || !(0..320).contains(&dx) || !(32..156).contains(&dy) {
                    continue;
                }
                let index = (dy * 320 + dx) as usize;
                sampled[index] = true;
                if opacity == 0.0 {
                    continue;
                }
                let mut rgb =
                    crate::legacy_colors::hue::rotate([pixel[0], pixel[1], pixel[2]], hue);
                rgb = crate::legacy_colors::tone::apply(rgb, tone);
                if flash[3] != 0 {
                    for (channel, overlay) in rgb.iter_mut().zip(flash) {
                        *channel = ((u32::from(*channel) * u32::from(255 - flash[3]) + 127) / 255
                            + (u32::from(overlay) * u32::from(flash[3]) / 256))
                            as u8;
                    }
                }
                expected[index] = [rgb[0], rgb[1], rgb[2], 255];
            }
        }
    }
    let mut pixels = expected
        .into_iter()
        .enumerate()
        .filter(|(i, _)| sampled[*i])
        .map(|(i, pixel)| (i as u32 % 320, i as u32 / 320, pixel))
        .collect::<Vec<_>>();
    assert!(pixels.len() > 1000);
    let shake_checks = label.starts_with("battle-shake-").then(|| {
        let system = world
            .resource::<AssetServer>()
            .load::<Image>("graphics/System/System.png");
        let skin = images.get(&system).unwrap();
        for x in 0..320 {
            let sx = if x < 8 {
                32 + x
            } else if x >= 312 {
                56 + x - 312
            } else {
                40 + (x - 8) % 16
            };
            for y in 0..8 {
                let pixel = rgba(skin, sx, y);
                if pixel[3] == 255 {
                    pixels.push((x, 160 + y, pixel));
                }
            }
        }
        crate::battle::smoke::shake::checks(world)
    });
    Some(Snapshot {
        pixels,
        shake_checks,
    })
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "battler ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        if let Some(checks) = &self.shake_checks {
            checks.fetch_add(1, Ordering::Relaxed);
        }
        info!(
            "battler flash/blink: {} GPU pixels verified",
            self.pixels.len()
        );
    }
}
