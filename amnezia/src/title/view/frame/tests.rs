use super::*;

fn skin() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: 160,
            height: 80,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    for (i, pixel) in image.data.as_mut().unwrap().chunks_exact_mut(4).enumerate() {
        pixel.copy_from_slice(&[(i % 160) as u8, (i / 160) as u8, 70, 255]);
    }
    image
}

#[test]
fn the_closed_window_is_transparent_and_the_first_frame_clips_both_corner_halves() {
    let skin = skin();
    let closed = render(&skin, 64, 0);
    assert!(
        closed
            .data
            .unwrap()
            .chunks_exact(4)
            .all(|pixel| pixel[3] == 0)
    );
    let first = render(&skin, 64, 1);
    for (y, source) in [(28, 0), (31, 3), (32, 28), (35, 31)] {
        assert_eq!(rgba(&first, 0, y), [32, source, 70, 255]);
        assert_eq!(rgba(&first, 63, y), [63, source, 70, 255]);
    }
    assert_eq!(rgba(&first, 0, 27)[3], 0);
    assert_eq!(rgba(&first, 0, 36)[3], 0);
}

#[test]
fn opening_crops_the_full_background_and_preserves_the_original_tiled_border_phase() {
    let skin = skin();
    for opened in 3..=8 {
        let image = render(&skin, 64, opened);
        assert_eq!(rgba(&image, 12, 32), [6, 16, 70, 255]);
        assert_eq!(rgba(&image, 0, 32), [32, 8, 70, 255]);
    }
    let full = render(&skin, 70, 8);
    for x in [8, 24, 40, 56] {
        assert_eq!(rgba(&full, x, 0), [48, 0, 70, 255]);
    }
    assert_eq!(rgba(&full, 62, 0), [56, 0, 70, 255]);
    assert_eq!(rgba(&full, 0, 8), [32, 16, 70, 255]);
    assert_eq!(rgba(&full, 0, 55), [32, 15, 70, 255]);
}

#[test]
fn transparent_frame_pixels_reveal_the_unmoved_blue_background() {
    let mut skin = skin();
    skin.data.as_mut().unwrap()[32 * 4 + 3] = 0;
    let image = render(&skin, 64, 1);
    assert_eq!(rgba(&image, 0, 28), [0, 14, 70, 255]);
}

#[test]
fn every_opening_phase_reuses_a_private_image_without_overwriting_the_system_skin() {
    use bevy::ecs::system::RunSystemOnce;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>();
    let world = app.world_mut();
    let system = world
        .resource::<AssetServer>()
        .load::<Image>("graphics/System/System.png");
    let source = skin();
    let original = source.data.clone();
    world
        .resource_mut::<Assets<Image>>()
        .insert(system.id(), source)
        .unwrap();
    let entity = world
        .spawn(WindowFrame {
            width: 64,
            opened: 1,
        })
        .id();
    world.run_system_once(rasterize).unwrap();
    let handle = world.get::<ImageNode>(entity).unwrap().image.clone();
    assert_ne!(handle, system);
    let count = world.resource::<Assets<Image>>().len();
    for opened in 2..=8 {
        world.get_mut::<WindowFrame>(entity).unwrap().opened = opened;
        world.run_system_once(rasterize).unwrap();
        assert_eq!(world.get::<ImageNode>(entity).unwrap().image, handle);
        assert_eq!(world.resource::<Assets<Image>>().len(), count);
        assert_eq!(
            world.resource::<Assets<Image>>().get(&system).unwrap().data,
            original
        );
    }
    let before = world
        .resource::<Assets<Image>>()
        .get(&handle)
        .unwrap()
        .data
        .clone();
    world
        .resource_mut::<Assets<Image>>()
        .get_mut(&system)
        .unwrap()
        .data
        .as_mut()
        .unwrap()
        .fill(0);
    world.write_message(AssetEvent::Modified { id: system.id() });
    world.run_system_once(rasterize).unwrap();
    assert_eq!(world.get::<ImageNode>(entity).unwrap().image, handle);
    assert_ne!(
        world.resource::<Assets<Image>>().get(&handle).unwrap().data,
        before
    );
}
