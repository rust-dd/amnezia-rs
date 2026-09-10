use super::*;
use crate::legacy_colors::{flash::SpriteFlash, hue::HueShift, tone::SpriteTone};

pub(crate) struct Snapshot(Vec<(u32, u32, [u8; 4])>);

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
    let mut expected = (0..320 * 160)
        .map(|i| {
            let pixel = rgba(backdrop, i % 320, i / 320);
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
        let left = base.x as i32 + 160 - source.width() as i32 / 2;
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
    let pixels = expected
        .into_iter()
        .enumerate()
        .filter(|(i, _)| sampled[*i])
        .map(|(i, pixel)| (i as u32 % 320, i as u32 / 320, pixel))
        .collect::<Vec<_>>();
    assert!(pixels.len() > 1000);
    Some(Snapshot(pixels))
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.0 {
            let actual = rgba(image, x * image.width() / 320, y * image.height() / 240);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "battler ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        info!("battler flash/blink: {} GPU pixels verified", self.0.len());
    }
}
