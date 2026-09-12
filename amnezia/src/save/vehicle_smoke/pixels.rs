use super::*;
use crate::{assets::resolve_png, tiles, vehicles::VehicleSprite, world::MainCamera};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
struct Checks(Arc<AtomicUsize>);

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checked: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let opacity = match label {
        "saved-vehicles-before" | "saved-vehicles-restored" => tiles::character_alpha(1),
        "saved-vehicles-ascent-resumed" | "saved-vehicles-landed" | "saved-vehicles-legacy" => 1.0,
        _ => return None,
    };
    world.init_resource::<Checks>();
    let camera = world
        .query_filtered::<&GlobalTransform, With<MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    let hero_visible = world
        .query_filtered::<&InheritedVisibility, With<Player>>()
        .single(world)
        .unwrap();
    assert!(!hero_visible.get());
    let mut query = world.query::<(
        &VehicleSprite,
        &Sprite,
        &GlobalTransform,
        &InheritedVisibility,
    )>();
    let sprites = query.iter(world).collect::<Vec<_>>();
    assert_eq!(sprites.len(), 3);
    let vehicles = world.resource::<Vehicles>();
    let data = world.resource::<MapData>();
    let mut pixels = Vec::new();
    for (id, sprite, transform, visible) in sprites {
        let vehicle = &vehicles.save.vehicles[id.0];
        let (name, index) = match id.0 {
            0 if label != "saved-vehicles-ascent-resumed" => ("Chara1", 2),
            0 => ("vehicle", 0),
            1 => ("vehicle", 1),
            _ => ("Vehicle", 7),
        };
        assert_eq!((vehicle.charset(), vehicle.index()), (name, index));
        assert!(visible.get());
        assert_eq!(sprite.color.alpha(), opacity);
        let source = world
            .resource::<AssetServer>()
            .load::<Image>(resolve_png("CharSet", name));
        assert_eq!(sprite.image, source);
        let (sx, sy) = tiles::charset_source(index, vehicle.dir, vehicle.frame);
        assert_eq!(sprite.rect, Some(Rect::new(sx, sy, sx + 24.0, sy + 32.0)));
        let expected =
            vehicles.pixel(10002 + id.0 as i32, data).unwrap() + Vec2::Y * tiles::CHAR_Y_OFFSET;
        assert!(
            transform
                .translation()
                .truncate()
                .abs_diff_eq(expected, 0.001)
        );
        let position = transform.translation() - camera;
        let left = first_covered_pixel(160.0 + position.x - 12.0);
        let top = first_covered_pixel(120.0 - position.y - 16.0);
        assert!(left + 24 <= 320 && top + 32 <= 240);
        let image = world.resource::<Assets<Image>>().get(&source).unwrap();
        let opacity = (opacity * 255.0).round() as u32;
        for y in 0..32 {
            for x in 0..24 {
                let source = image
                    .get_color_at(sx as u32 + x, sy as u32 + y)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                let mut expected = [0, 0, 0, 255];
                if source[3] != 0 {
                    assert_eq!(source[3], 255);
                    for channel in 0..3 {
                        expected[channel] =
                            ((u32::from(source[channel]) * opacity + 127) / 255) as u8;
                    }
                }
                pixels.push((left + x, top + y, expected));
            }
        }
    }
    Some(Snapshot {
        pixels,
        checked: world.resource::<Checks>().0.clone(),
    })
}

fn first_covered_pixel(edge: f32) -> u32 {
    (edge - 0.5).ceil() as u32
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "saved vehicle ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checked.fetch_add(1, Ordering::Relaxed);
        info!("saved vehicles: 2304 original charset, opacity and background pixels verified");
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().0.load(Ordering::Relaxed), 5);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_pixel_edges_include_the_native_pixel_center_on_the_boundary() {
        for (edge, expected) in [
            (121.0, 121),
            (121.499, 121),
            (121.5, 121),
            (121.501, 122),
            (122.0, 122),
            (218.25, 218),
        ] {
            assert_eq!(first_covered_pixel(edge), expected);
        }
    }
}
