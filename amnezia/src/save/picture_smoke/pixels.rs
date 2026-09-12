use super::*;

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 3])>,
    verified: Arc<AtomicU32>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if label != "save-pictures-restored" {
        return None;
    }
    let pictures = current(world);
    let camera = world
        .query_filtered::<&Transform, With<crate::world::MainCamera>>()
        .single(world)
        .unwrap()
        .translation
        .truncate();
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("Picture", "Cross"));
    let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
    assert_eq!(image.size(), UVec2::splat(3));
    let mut pixels = Vec::new();
    for picture in &pictures[1..] {
        assert_eq!(picture.name, "Cross");
        let center = picture.world_anchor.map_or_else(
            || Vec2::new(picture.visual.x, picture.visual.y),
            |anchor| Vec2::new(anchor[0] - camera.x + 160.0, camera.y - anchor[1] + 120.0),
        );
        for row in 0..3 {
            for col in 0..3 {
                let rgb = image
                    .get_color_at(col, row)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                pixels.push((
                    (center.x - 10.0 + col as f32 * 8.0).round() as u32,
                    (center.y - 10.0 + row as f32 * 8.0).round() as u32,
                    [rgb[0], rgb[1], rgb[2]],
                ));
            }
        }
    }
    Some(Snapshot {
        pixels,
        verified: world.resource::<Fixture>().pixels.clone(),
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
                    "saved picture ({x},{y}): expected {expected:?}, got {actual:?}"
                );
            }
        }
        self.verified.fetch_add(1, Ordering::SeqCst);
        info!(
            "saved pictures: 18 original screen-pinned and map-anchored graphic samples verified"
        );
    }
}
