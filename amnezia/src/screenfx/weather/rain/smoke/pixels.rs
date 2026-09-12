use super::*;

pub(crate) struct Snapshot {
    drops: Vec<model::Drop>,
    strength: i32,
    tone: [f32; 4],
    offset: [i32; 2],
    camera_x: f32,
    checked: Arc<AtomicU32>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !label.starts_with("weather-pixels-") {
        return None;
    }
    assert_eq!(*world.resource::<Weather>(), Weather::Rain);
    let checks = world.resource::<Checks>();
    let position = world
        .resource::<crate::player::CameraPan>()
        .position
        .unwrap();
    let pan = Vec2::new(
        (checks.camera.x - position.x).rem_euclid(320.0),
        (position.y - checks.camera.y).rem_euclid(160.0),
    );
    assert!((world.resource::<Scroll>().pan - pan).abs().max_element() < 0.001);
    let shake = world.resource::<crate::screenfx::Fx>().shake_offset;
    if label.ends_with("pan-shake") {
        assert_ne!(shake, Vec2::ZERO);
        assert_ne!(position, checks.camera);
    }
    let checked = checks.pixels.clone();
    let anchor = checks.anchor;
    let camera_x = world
        .query_filtered::<&Transform, With<MainCamera>>()
        .single(world)
        .unwrap()
        .translation
        .x
        - anchor.x;
    Some(Snapshot {
        drops: world.resource::<Rain>().drops.clone(),
        strength: world.resource::<WeatherStrength>().0,
        tone: world.resource::<TintState>().tone(),
        offset: [
            -pan.x as i32 + shake.x as i32,
            -pan.y as i32 + shake.y as i32,
        ],
        camera_x,
        checked,
    })
}

fn tone(rgb: [u8; 3], values: [f32; 4]) -> [u8; 3] {
    let levels = values.map(|value| (value * 128.0 / 100.0).trunc().clamp(0.0, 255.0) as i32);
    let rgb = rgb.map(i32::from);
    let grey = (rgb[0] * 19595 + rgb[1] * 38470 + rgb[2] * 7471) / 65536;
    let saturation = if levels[3] <= 128 {
        levels[3] * 8
    } else {
        1024 + (levels[3] - 128) * 16
    };
    std::array::from_fn(|index| {
        let channel = ((grey * 1024 + (rgb[index] - grey) * saturation) >> 10).clamp(0, 255);
        let level = levels[index];
        if level <= 128 {
            (2 * level * channel / 255).clamp(0, 255) as u8
        } else {
            (255 - 2 * (255 - level) * (255 - channel) / 255).clamp(0, 255) as u8
        }
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        let mut mask = vec![0_u32; 320 * 160];
        for drop in self
            .drops
            .iter()
            .take([20, 60, 100][self.strength as usize])
        {
            if drop.life > 12 {
                continue;
            }
            let alpha = (5 + self.strength as u32) * u32::from(drop.life);
            for row in 0..24 {
                for ox in [0, -320] {
                    for oy in [0, -160] {
                        if ox != 0 && drop.x + 6 <= 320 || oy != 0 && drop.y + 24 <= 160 {
                            continue;
                        }
                        let x = drop.x + 5 - row / 4 + ox;
                        let y = drop.y + row + oy;
                        if !(0..320).contains(&x) || !(0..160).contains(&y) {
                            continue;
                        }
                        let old = &mut mask[(y * 320 + x) as usize];
                        *old = alpha + (*old * (255 - alpha) + 127) / 255;
                    }
                }
            }
        }
        let rain_color = tone([255; 3], self.tone);
        let mut checked = 0;
        let mut rainy = 0;
        for y in 0..240 {
            for x in 0..320 {
                let source_x = (x as f32 + self.camera_x + 0.5).floor() as i32;
                if [0, 107, 214, 321]
                    .iter()
                    .any(|edge| (source_x - edge).abs() < 2)
                {
                    continue;
                }
                let actual = crate::display::smoke::pixel_at(image, x as u32, y as u32);
                let expected = if (248..272).contains(&x) && (28..52).contains(&y) {
                    UI
                } else {
                    let base = if (0..321).contains(&source_x) {
                        BASES[(source_x / 107) as usize]
                    } else {
                        BASES[0]
                    };
                    let base = tone(base, self.tone);
                    let alpha = mask[((y + self.offset[1]).rem_euclid(160) * 320
                        + (x + self.offset[0]).rem_euclid(320))
                        as usize];
                    rainy += usize::from(alpha != 0);
                    std::array::from_fn(|channel| {
                        ((u32::from(base[channel]) * (255 - alpha)
                            + u32::from(rain_color[channel]) * alpha)
                            / 255) as u8
                    })
                };
                assert!(
                    actual[..3]
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| a.abs_diff(b) <= 1),
                    "rain pixel ({x},{y}): {actual:?}, expected {expected:?}"
                );
                checked += 1;
            }
        }
        assert!(checked > 74000 && rainy > 100);
        self.checked.fetch_add(1, Ordering::SeqCst);
        info!(
            "rain: {checked} reference bitmap, alpha, tone, tiling and unchanged UI pixels verified ({rainy} wet pixels)"
        );
    }
}
