use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

fn source(app: &mut App, rgb: [u8; 3]) -> Handle<Image> {
    let mut data = Vec::new();
    for alpha in [255, 128, 0] {
        data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], alpha]);
    }
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new(
            Extent3d {
                width: 3,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ))
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<TintState>()
        .init_resource::<Assets<Image>>();
    register(&mut app);
    app.world_mut()
        .resource_mut::<TintState>()
        .set_tone([50.0, 100.0, 150.0, 0.0]);
    app
}

fn pixels(app: &App, entity: Entity) -> Vec<u8> {
    let sprite = app.world().get::<Sprite>(entity).unwrap();
    app.world()
        .resource::<Assets<Image>>()
        .get(&sprite.image)
        .unwrap()
        .data
        .clone()
        .unwrap()
}

#[test]
fn shared_world_images_keep_source_and_sprite_opacity_without_tinting_the_foreground() {
    let mut app = app();
    let source = source(&mut app, [32, 156, 0]);
    let originals = app
        .world()
        .resource::<Assets<Image>>()
        .get(&source)
        .unwrap()
        .data
        .clone();
    let sprites = (0..100)
        .map(|_| {
            app.world_mut()
                .spawn(Sprite {
                    image: source.clone(),
                    color: Color::WHITE.with_alpha(0.4),
                    ..default()
                })
                .id()
        })
        .collect::<Vec<_>>();
    let front = app
        .world_mut()
        .spawn((
            Sprite::from_image(source.clone()),
            RenderLayers::layer(crate::screenfx::PICTURE_LAYER),
        ))
        .id();
    app.update();
    let tinted = app.world().get::<Sprite>(sprites[0]).unwrap().image.clone();
    for entity in sprites {
        let sprite = app.world().get::<Sprite>(entity).unwrap();
        assert_eq!(sprite.image, tinted);
        assert_eq!(sprite.color.alpha(), 0.4);
        assert_eq!(
            pixels(&app, entity),
            [50, 101, 179, 255, 50, 101, 179, 128, 32, 156, 0, 0]
        );
    }
    assert_eq!(app.world().get::<Sprite>(front).unwrap().image, source);
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&source)
            .unwrap()
            .data,
        originals
    );
    assert_eq!(app.world().resource::<Assets<Image>>().len(), 2);
}

#[test]
fn tone_changes_reuse_the_shared_texture_and_neutral_restores_the_original() {
    let mut app = app();
    let source = source(&mut app, [32, 156, 0]);
    let entity = app
        .world_mut()
        .spawn(Sprite::from_image(source.clone()))
        .id();
    app.update();
    let tinted = app.world().get::<Sprite>(entity).unwrap().image.clone();
    app.world_mut()
        .resource_mut::<TintState>()
        .set_tone([200.0, 200.0, 200.0, 100.0]);
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(app.world().get::<Sprite>(entity).unwrap().image, tinted);
    assert_eq!(
        pixels(&app, entity),
        [255, 255, 255, 255, 255, 255, 255, 128, 32, 156, 0, 0]
    );
    app.world_mut()
        .resource_mut::<TintState>()
        .set_tone([100.0; 4]);
    app.update();
    assert_eq!(app.world().get::<Sprite>(entity).unwrap().image, source);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), 2);
}

#[test]
fn changing_a_charset_while_tinted_does_not_keep_or_double_tint_the_old_source() {
    let mut app = app();
    let first = source(&mut app, [32, 156, 0]);
    let next = source(&mut app, [64, 128, 192]);
    let entity = app.world_mut().spawn(Sprite::from_image(first)).id();
    app.update();
    app.world_mut().get_mut::<Sprite>(entity).unwrap().image = next.clone();
    app.update();
    let expected = pixels(&app, entity);
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(pixels(&app, entity), expected);
    app.world_mut()
        .resource_mut::<TintState>()
        .set_tone([100.0; 4]);
    app.update();
    assert_eq!(app.world().get::<Sprite>(entity).unwrap().image, next);
}

#[test]
fn target_flash_is_after_tone_and_does_not_modify_shared_images_or_alpha() {
    let mut app = app();
    let source = source(&mut app, [32, 156, 0]);
    let plain = app
        .world_mut()
        .spawn(Sprite::from_image(source.clone()))
        .id();
    let flashed = app
        .world_mut()
        .spawn((
            Sprite::from_image(source.clone()),
            SpriteFlash([248, 160, 80, 96]),
        ))
        .id();
    app.update();
    assert_eq!(
        pixels(&app, flashed),
        [124, 123, 142, 255, 124, 123, 142, 128, 32, 156, 0, 0]
    );
    let private = app.world().get::<Sprite>(flashed).unwrap().image.clone();
    app.world_mut()
        .resource_mut::<TintState>()
        .set_tone([200.0, 200.0, 200.0, 100.0]);
    app.update();
    assert_eq!(&pixels(&app, flashed)[..4], &[252, 219, 189, 255]);
    assert_eq!(app.world().get::<Sprite>(flashed).unwrap().image, private);
    assert_eq!(&pixels(&app, plain)[..4], &[255; 4]);
    app.world_mut().get_mut::<SpriteFlash>(flashed).unwrap().0 = [0; 4];
    app.update();
    assert_eq!(
        app.world().get::<Sprite>(flashed).unwrap().image,
        app.world().get::<Sprite>(plain).unwrap().image
    );
}

#[test]
fn bush_children_keep_the_toned_parent_image_without_being_tinted_twice() {
    let mut app = app();
    let source = source(&mut app, [32, 156, 0]);
    let parent = app.world_mut().spawn(Sprite::from_image(source)).id();
    app.update();
    let sprite = app.world().get::<Sprite>(parent).unwrap().clone();
    let bottom = app
        .world_mut()
        .spawn((sprite, crate::world::BushBottom, ChildOf(parent)))
        .id();
    app.update();
    assert_eq!(pixels(&app, parent), pixels(&app, bottom));
    assert!(app.world().get::<WorldImage>(bottom).is_none());
}

#[test]
fn changing_render_layers_restores_the_original_and_map_changes_release_the_cache() {
    let mut app = app();
    let source = source(&mut app, [32, 156, 0]);
    let entity = app
        .world_mut()
        .spawn(Sprite::from_image(source.clone()))
        .id();
    app.update();
    app.world_mut()
        .entity_mut(entity)
        .insert(RenderLayers::layer(crate::screenfx::PICTURE_LAYER));
    app.update();
    assert_eq!(app.world().get::<Sprite>(entity).unwrap().image, source);
    assert!(app.world().get::<WorldImage>(entity).is_none());
    app.world_mut().write_message(crate::world::MapChanged);
    app.update();
    assert!(app.world().resource::<Cache>().0.is_empty());
}
