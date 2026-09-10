use super::*;

#[test]
fn output_preserves_native_pixels_aspect_ratio_and_small_windows() {
    for (size, scale) in [
        (UVec2::new(320, 240), 1.0),
        (UVec2::new(960, 720), 3.0),
        (UVec2::new(1920, 1080), 4.0),
        (UVec2::new(720, 1080), 2.0),
        (UVec2::new(1440, 1080), 4.0),
        (UVec2::new(160, 120), 0.5),
    ] {
        assert_eq!(output_scale(size), scale);
        let Projection::Orthographic(camera) = projection(size) else {
            unreachable!()
        };
        let bevy::camera::ScalingMode::Fixed { width, height } = camera.scaling_mode else {
            unreachable!()
        };
        assert_eq!(width * scale, size.x as f32);
        assert_eq!(height * scale, size.y as f32);
    }
    assert!(output_scale(UVec2::ZERO).is_finite());
    assert_eq!(
        center_offset(UVec2::new(1001, 751)),
        Vec2::new(-0.5 / 3.0, 0.5 / 3.0)
    );
}

#[test]
fn every_game_layer_shares_a_fixed_canvas_while_output_size_changes() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Mesh>>()
        .add_plugins(DisplayPlugin);
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(960, 720),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let cameras = (0..4)
        .map(|order| {
            app.world_mut()
                .spawn((Camera2d, Camera { order, ..default() }))
                .id()
        })
        .collect::<Vec<_>>();
    app.update();
    let canvas = app.world().resource::<Canvas>().0.clone();
    for camera in cameras {
        assert!(
            matches!(app.world().get::<RenderTarget>(camera), Some(RenderTarget::Image(image)) if image.handle == canvas)
        );
        assert_eq!(app.world().get::<Msaa>(camera), Some(&Msaa::Off));
    }
    assert_eq!(app.world().resource::<UiScale>().0, 1.0 / 3.0);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .resolution
        .set(1280.0, 720.0);
    let later = app.world_mut().spawn(Camera2d).id();
    app.update();
    assert!(
        matches!(app.world().get::<RenderTarget>(later), Some(RenderTarget::Image(image)) if image.handle == canvas)
    );
    assert_eq!(app.world().resource::<Canvas>().0, canvas);
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&canvas)
            .unwrap()
            .size(),
        NATIVE_SIZE
    );
    let world = app.world_mut();
    let projection = world
        .query_filtered::<&Projection, With<PresentationCamera>>()
        .single(world)
        .unwrap();
    let Projection::Orthographic(projection) = projection else {
        unreachable!()
    };
    assert!(
        matches!(projection.scaling_mode, bevy::camera::ScalingMode::Fixed { width, height } if (width - 1280.0 / 3.0).abs() < 1e-4 && height == 240.0)
    );
}
