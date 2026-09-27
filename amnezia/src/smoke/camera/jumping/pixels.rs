use super::*;

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 3])>,
    verified: Arc<AtomicU32>,
}

pub(in crate::smoke) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let scroll = match label {
        "camera-jump-before" => 8.25_f32,
        "camera-jump-landed" => 9.25,
        "camera-jump-pan-finished" => 16.0,
        _ => return None,
    };
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("Picture", "Cross"));
    let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
    assert_eq!(image.size(), UVec2::splat(3));
    let mut pixels = Vec::new();
    for center_x in [96.0 - scroll, 224.0] {
        for row in 0..3 {
            for col in 0..3 {
                let rgb = image
                    .get_color_at(col, row)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                pixels.push((
                    (center_x.trunc() - 6.0 + col as f32 * 8.0) as u32,
                    54 + row * 8,
                    [rgb[0], rgb[1], rgb[2]],
                ));
            }
        }
    }
    Some(Snapshot {
        pixels,
        verified: world.resource::<Probe>().pixels.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        assert_eq!(self.pixels.len(), 18);
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            for channel in 0..3 {
                assert!(
                    actual[channel].abs_diff(expected[channel]) <= 1,
                    "jump picture ({x},{y}): expected {expected:?}, got {actual:?}"
                );
            }
        }
        self.verified.fetch_add(1, Ordering::SeqCst);
        info!("jump camera: 18 original map-fixed and screen-fixed picture samples verified");
    }
}
