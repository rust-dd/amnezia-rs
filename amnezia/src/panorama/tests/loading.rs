use super::*;
use bevy::image::{CompressedImageFormats, ImageLoader};
use std::time::{Duration, Instant};

fn app() -> App {
    app_with(|_| {})
}

fn app_with(configure: impl FnOnce(&mut App)) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: crate::assets::asset_root().into(),
            ..default()
        },
        ImagePlugin::default_nearest(),
        PanoramaPlugin,
    ))
    .register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE))
    .init_resource::<crate::player::CameraPan>()
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs_f64(1.0 / 60.0),
    ));
    let mut map = MapData::for_test(20, 16);
    map.map_id = 94;
    map.panorama = Some(PanoramaDef::from_command(
        "Sky".into(),
        &[1, 1, 1, 8, 1, -6],
    ));
    app.insert_resource(map);
    app.world_mut().spawn((MainCamera, Transform::default()));
    configure(&mut app);
    app.finish();
    app.cleanup();
    app
}

fn wait_for(app: &mut App, ready: impl Fn(&mut World) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.update();
        if ready(app.world_mut()) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the original background did not become ready"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub(super) fn loaded_without_tiles() -> App {
    loaded_without_tiles_with(|_| {})
}

pub(super) fn loaded_without_tiles_with(configure: impl FnOnce(&mut App)) -> App {
    let mut app = app_with(configure);
    app.world_mut().resource_mut::<MapData>().panorama = None;
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::ZERO,
    ));
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<Image>(resolve_png("Panorama", "Sky"));
    wait_for(&mut app, |world| {
        world.resource::<Assets<Image>>().contains(&handle)
    });
    app.insert_resource(Panorama::default());
    app.world_mut().resource_mut::<MapData>().panorama =
        Some(PanoramaDef::from_command("Sky".into(), &[1, 1, 0, 0, 0, 0]));
    app.world_mut()
        .query_filtered::<&mut Transform, With<MainCamera>>()
        .single_mut(app.world_mut())
        .unwrap()
        .translation
        .y = -8.0;
    app.update();
    app
}

pub(super) fn change(app: &mut App, name: &str, params: &[i32]) {
    app.world_mut()
        .resource_scope(|world, mut panorama: Mut<Panorama>| {
            world.resource_scope(|world, mut camera: Mut<crate::player::CameraPan>| {
                panorama.change(
                    world.resource::<MapData>(),
                    &mut camera,
                    PanoramaDef::from_command(name.into(), params),
                );
            });
        });
}

#[test]
fn an_uncached_original_background_stays_alive_until_it_can_spawn_visible_tiles() {
    let mut app = app();
    wait_for(&mut app, |world| {
        world.query::<&PanoramaTile>().iter(world).count() != 0
    });
    for _ in 0..10 {
        app.update();
    }
    let world = app.world_mut();
    let tiles = world
        .query::<(&PanoramaTile, &Sprite, &Transform)>()
        .iter(world)
        .collect::<Vec<_>>();
    assert_eq!(tiles.len(), 9);
    for (_, sprite, transform) in tiles {
        assert_eq!(
            sprite.image.path().unwrap().path().file_name().unwrap(),
            "Sky.png"
        );
        assert_eq!(
            world
                .resource::<Assets<Image>>()
                .get(&sprite.image)
                .unwrap()
                .size(),
            UVec2::new(640, 480)
        );
        assert!(transform.translation.is_finite());
    }
}

#[test]
fn destination_preparation_owns_the_image_before_tiles_exist_and_replaces_stale_requests() {
    let mut app = app();
    assert!(app.world().resource::<BackgroundImage>().image().is_none());
    prepare(app.world_mut());
    let first = app
        .world()
        .resource::<BackgroundImage>()
        .image()
        .unwrap()
        .clone();
    assert_eq!(first.path().unwrap().path().file_name().unwrap(), "Sky.png");
    assert_eq!(
        app.world_mut()
            .query::<&PanoramaTile>()
            .iter(app.world())
            .count(),
        0
    );
    prepare(app.world_mut());
    assert_eq!(
        app.world().resource::<BackgroundImage>().image(),
        Some(&first)
    );
    change(&mut app, "Morning1", &[1, 1, 0, 0, 0, 0]);
    prepare(app.world_mut());
    let next = app.world().resource::<BackgroundImage>().image().unwrap();
    assert_ne!(next, &first);
    assert_eq!(
        next.path().unwrap().path().file_name().unwrap(),
        "Morning1.png"
    );
    app.world_mut().resource_mut::<MapData>().panorama = None;
    change(&mut app, "", &[0; 6]);
    prepare(app.world_mut());
    assert!(app.world().resource::<BackgroundImage>().image().is_none());
}

#[test]
fn destination_preparation_initializes_backgrounds_without_spending_the_map_tick() {
    let mut app = loaded_without_tiles();
    change(&mut app, "Sky", &[1, 1, 1, -1, 1, 1]);
    let before = app.world().resource::<Panorama>().clone();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(1));
    for _ in 0..3 {
        prepare(app.world_mut());
        assert_eq!(*app.world().resource::<Panorama>(), before);
    }
    advance(app.world_mut());
    assert_eq!(app.world().resource::<Panorama>().clock.frame, 60);
    assert_ne!(*app.world().resource::<Panorama>(), before);
}

#[test]
fn a_loaded_background_is_tiled_at_the_right_position_on_its_first_visible_frame() {
    let mut app = loaded_without_tiles();
    let world = app.world_mut();
    let (_, transform) = world
        .query::<(&PanoramaTile, &Transform)>()
        .iter(world)
        .find(|(tile, _)| tile.0 == 0 && tile.1 == 0)
        .unwrap();
    assert_eq!(transform.translation, Vec3::new(160.0, -600.0, -10.0));
}

#[test]
fn changing_scroll_parameters_keeps_the_existing_background_entities() {
    let mut app = loaded_without_tiles();
    app.update();
    let before = app
        .world_mut()
        .query_filtered::<Entity, With<PanoramaTile>>()
        .iter(app.world())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(before.len(), 9);
    change(&mut app, "Sky", &[1, 1, 1, 6, 1, -6]);
    app.update();
    let after = app
        .world_mut()
        .query_filtered::<Entity, With<PanoramaTile>>()
        .iter(app.world())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(after, before);
}

#[test]
fn replacing_and_clearing_a_background_preserves_then_releases_its_visible_tiles() {
    let mut app = loaded_without_tiles();
    change(&mut app, "Ground", &[1, 1, 0, 0, 0, 0]);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&PanoramaTile>()
            .iter(app.world())
            .count(),
        9
    );
    wait_for(&mut app, |world| {
        world.query::<&PanoramaTile>().iter(world).count() == 414
    });
    app.world_mut().resource_mut::<MapData>().panorama = None;
    change(&mut app, "", &[0; 6]);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&PanoramaTile>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(app.world().resource::<BackgroundImage>().0.is_none());
}

#[test]
fn an_empty_map_background_does_not_request_an_empty_graphic() {
    let mut app = app();
    app.world_mut().resource_mut::<MapData>().panorama =
        Some(PanoramaDef::from_command(String::new(), &[0; 6]));
    app.update();
    assert!(app.world().resource::<Panorama>().definition.is_none());
    assert!(app.world().resource::<BackgroundImage>().0.is_none());
    assert_eq!(
        app.world_mut()
            .query::<&PanoramaTile>()
            .iter(app.world())
            .count(),
        0
    );
}
