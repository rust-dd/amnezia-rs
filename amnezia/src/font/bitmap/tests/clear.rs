use super::*;
use bevy::ecs::system::RunSystemOnce;

#[test]
fn clear_rect_clips_negative_empty_and_extreme_bounds_without_overflow() {
    let font = BitmapFont::from_id(0);
    let mut text = PixelText {
        size: UVec2::new(12, 16),
        runs: vec![Run::new("Őg", 0, 0, 0)],
    };
    let original = font.render(&text, &system());
    for (x, y, width, height) in [
        (-3, -2, 7, 8),
        (9, 10, 20, 20),
        (0, 0, 0, 16),
        (0, 0, 12, 0),
        (i32::MAX, i32::MAX, u32::MAX, u32::MAX),
        (-2, -2, u32::MAX, u32::MAX),
    ] {
        text.runs.truncate(1);
        text.runs.push(Run::clear(x, y, width, height));
        let image = font.render(&text, &system());
        for py in 0..16 {
            for px in 0..12 {
                let erased = (i64::from(x)..i64::from(x) + i64::from(width))
                    .contains(&i64::from(px))
                    && (i64::from(y)..i64::from(y) + i64::from(height)).contains(&i64::from(py));
                let expected = if erased {
                    [0; 4]
                } else {
                    original
                        .get_color_at(px, py)
                        .unwrap()
                        .to_srgba()
                        .to_u8_array()
                };
                assert_eq!(
                    image.get_color_at(px, py).unwrap().to_srgba().to_u8_array(),
                    expected,
                    "{x},{y},{width},{height}: ({px},{py})"
                );
            }
        }
    }
}

#[test]
fn clear_rect_preserves_draw_order_and_later_text() {
    let font = BitmapFont::from_id(0);
    let mut text = PixelText {
        size: UVec2::new(20, 16),
        runs: vec![
            Run::new("A", 0, 0, 0),
            Run::clear(0, 0, 20, 16),
            Run::new("B", 0, 0, 1),
        ],
    };
    let expected = font.render(
        &PixelText {
            size: text.size,
            runs: vec![Run::new("B", 0, 0, 1)],
        },
        &system(),
    );
    assert_eq!(font.render(&text, &system()).data, expected.data);
    text.runs.push(Run::clear(0, 0, 20, 16));
    assert!(
        font.render(&text, &system())
            .data
            .unwrap()
            .iter()
            .all(|v| *v == 0)
    );
}

#[test]
fn clear_rect_changes_invalidate_the_cache_without_replacing_its_image() {
    let mut world = World::new();
    world.init_resource::<Assets<Image>>();
    let system = world.resource_mut::<Assets<Image>>().add(system());
    world.insert_resource(BitmapFont::from_id(0));
    world.insert_resource(Palette {
        source: system,
        revision: 0,
    });
    world.init_resource::<Messages<AssetEvent<Image>>>();
    let entity = world
        .spawn(PixelText {
            size: UVec2::new(20, 16),
            runs: vec![Run::new("A", 0, 0, 0), Run::clear(0, 0, 0, 16)],
        })
        .id();
    world.run_system_once(rasterize).unwrap();
    let handle = world.get::<ImageNode>(entity).unwrap().image.clone();
    let before = world
        .resource::<Assets<Image>>()
        .get(&handle)
        .unwrap()
        .data
        .clone();
    world.get_mut::<PixelText>(entity).unwrap().runs[1] = Run::clear(0, 0, 20, 16);
    world.run_system_once(rasterize).unwrap();
    assert_eq!(world.get::<ImageNode>(entity).unwrap().image, handle);
    let after = &world.resource::<Assets<Image>>().get(&handle).unwrap().data;
    assert_ne!(after, &before);
    assert!(after.as_ref().unwrap().iter().all(|v| *v == 0));
    assert_eq!(world.resource::<Assets<Image>>().len(), 2);
}
