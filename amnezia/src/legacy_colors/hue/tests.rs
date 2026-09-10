use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[test]
fn hue_matches_reference_integer_hsl_including_full_revolution_rounding() {
    let degrees = [30, 60, 120, 180, 330, -30, 360];
    for (source, expected) in [
        (
            [255, 0, 0],
            [
                [253, 126, 0],
                [252, 253, 0],
                [0, 253, 0],
                [0, 252, 253],
                [253, 0, 125],
                [253, 0, 125],
                [253, 0, 0],
            ],
        ),
        (
            [32, 156, 0],
            [
                [0, 155, 46],
                [0, 155, 123],
                [0, 30, 155],
                [123, 0, 155],
                [108, 155, 0],
                [108, 155, 0],
                [30, 155, 0],
            ],
        ),
        (
            [64, 128, 192],
            [
                [64, 64, 191],
                [127, 64, 191],
                [191, 64, 127],
                [191, 127, 64],
                [64, 190, 191],
                [64, 190, 191],
                [64, 127, 191],
            ],
        ),
        (
            [3, 2, 1],
            [
                [2, 3, 1],
                [1, 3, 1],
                [1, 3, 2],
                [1, 1, 3],
                [3, 1, 1],
                [3, 1, 1],
                [3, 2, 1],
            ],
        ),
    ] {
        assert_eq!(rotate(source, 0), source);
        for (degrees, expected) in degrees.into_iter().zip(expected) {
            assert_eq!(rotate(source, degrees), expected, "{source:?}, {degrees}");
        }
    }
    for gray in 0..=255 {
        for degrees in [-720, -330, 30, 180, 330, 720] {
            assert_eq!(rotate([gray; 3], degrees), [gray; 3]);
        }
    }
}

#[test]
fn sprites_share_cached_hue_variants_without_mutating_source_or_alpha() {
    let mut app = App::new();
    app.init_resource::<Assets<Image>>();
    register(&mut app);
    let source = Image::new(
        Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![32, 156, 0, 255, 12, 34, 56, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(source.clone());
    let sprites = (0..2)
        .map(|_| {
            app.world_mut()
                .spawn((
                    Sprite::from_image(handle.clone()),
                    HueShift {
                        original: handle.clone(),
                        degrees: 120,
                    },
                ))
                .id()
        })
        .collect::<Vec<_>>();
    app.update();
    let first = app.world().get::<Sprite>(sprites[0]).unwrap().image.clone();
    assert_ne!(first, handle);
    assert_eq!(first, app.world().get::<Sprite>(sprites[1]).unwrap().image);
    let images = app.world().resource::<Assets<Image>>();
    assert_eq!(images.get(&handle).unwrap().data, source.data);
    assert_eq!(
        images.get(&first).unwrap().data.as_deref().unwrap(),
        [0, 30, 155, 255, 12, 34, 56, 0]
    );
    assert_eq!(app.world().resource::<Cache>().0.len(), 1);
    app.world_mut()
        .get_mut::<HueShift>(sprites[0])
        .unwrap()
        .degrees = 330;
    app.update();
    assert_eq!(app.world().resource::<Cache>().0.len(), 2);
    app.world_mut()
        .get_mut::<HueShift>(sprites[0])
        .unwrap()
        .degrees = 0;
    app.update();
    assert_eq!(app.world().get::<Sprite>(sprites[0]).unwrap().image, handle);
    assert_eq!(app.world().get::<Sprite>(sprites[1]).unwrap().image, first);
}

#[test]
fn sprites_wait_for_asynchronous_image_loading() {
    let mut app = App::new();
    app.init_resource::<Assets<Image>>();
    register(&mut app);
    let original = app.world().resource::<Assets<Image>>().reserve_handle();
    let entity = app
        .world_mut()
        .spawn((
            Sprite::from_image(original.clone()),
            HueShift {
                original: original.clone(),
                degrees: 180,
            },
        ))
        .id();
    app.update();
    assert!(app.world().resource::<Cache>().0.is_empty());
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(original.id(), Image::default())
        .unwrap();
    app.update();
    assert_ne!(app.world().get::<Sprite>(entity).unwrap().image, original);
}
