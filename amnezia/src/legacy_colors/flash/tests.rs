use super::*;
use crate::legacy_colors::hue::{self, HueShift};
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[test]
fn flash_blends_image_pixels_after_hue_and_reuses_its_private_texture() {
    let mut app = App::new();
    app.init_resource::<Assets<Image>>();
    hue::register(&mut app);
    let original = Image::new(
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
    let source = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(original.clone());
    let entity = app
        .world_mut()
        .spawn((
            Sprite::from_image(source.clone()),
            HueShift {
                original: source.clone(),
                degrees: 120,
            },
            SpriteFlash([248, 248, 248, 192]),
        ))
        .id();
    app.update();
    let texture = app.world().get::<Sprite>(entity).unwrap().image.clone();
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&texture)
            .unwrap()
            .data
            .as_deref(),
        Some([186, 193, 224, 255, 12, 34, 56, 0].as_slice()),
    );
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&source)
            .unwrap()
            .data,
        original.data
    );
    app.world_mut().get_mut::<SpriteFlash>(entity).unwrap().0 = [248, 0, 0, 96];
    app.update();
    assert_eq!(app.world().get::<Sprite>(entity).unwrap().image, texture);
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&texture)
            .unwrap()
            .data
            .as_deref(),
        Some([93, 19, 97, 255, 12, 34, 56, 0].as_slice()),
    );
    app.world_mut().get_mut::<SpriteFlash>(entity).unwrap().0[3] = 0;
    app.update();
    let hue = app.world().get::<Sprite>(entity).unwrap().image.clone();
    assert_ne!(hue, texture);
    assert_ne!(hue, source);
    app.world_mut().get_mut::<HueShift>(entity).unwrap().degrees = 0;
    app.update();
    assert_eq!(app.world().get::<Sprite>(entity).unwrap().image, source);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), 3);
}

#[test]
fn unloaded_flash_sources_do_not_allocate_or_recolor_other_sprites() {
    let mut app = App::new();
    app.init_resource::<Assets<Image>>();
    hue::register(&mut app);
    let source = app.world().resource::<Assets<Image>>().reserve_handle();
    let mut entities = Vec::new();
    for _ in 0..2 {
        entities.push(
            app.world_mut()
                .spawn((
                    Sprite::from_image(source.clone()),
                    HueShift {
                        original: source.clone(),
                        degrees: 0,
                    },
                ))
                .id(),
        );
    }
    app.world_mut()
        .entity_mut(entities[0])
        .insert(SpriteFlash([248, 248, 248, 192]));
    app.update();
    assert!(app.world().resource::<Assets<Image>>().is_empty());
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(source.id(), Image::default())
        .unwrap();
    app.update();
    assert_ne!(
        app.world().get::<Sprite>(entities[0]).unwrap().image,
        source
    );
    assert_eq!(
        app.world().get::<Sprite>(entities[1]).unwrap().image,
        source
    );
}

#[test]
fn effect_textures_never_overwrite_bevys_shared_default_image() {
    let mut app = App::new();
    app.init_resource::<Assets<Image>>();
    hue::register(&mut app);
    let default_image = Image::default();
    let mut images = app.world_mut().resource_mut::<Assets<Image>>();
    images
        .insert(Handle::<Image>::default().id(), default_image.clone())
        .unwrap();
    let source = images.add(default_image.clone());
    let entities = [[248, 0, 0, 192], [0, 248, 0, 96]].map(|color| {
        app.world_mut()
            .spawn((
                Sprite::from_image(source.clone()),
                HueShift {
                    original: source.clone(),
                    degrees: 0,
                },
                SpriteFlash(color),
            ))
            .id()
    });
    app.update();
    let textures = entities.map(|entity| app.world().get::<Sprite>(entity).unwrap().image.clone());
    assert_ne!(textures[0], Handle::<Image>::default());
    assert_ne!(textures[1], Handle::<Image>::default());
    assert_ne!(textures[0], textures[1]);
    let images = app.world().resource::<Assets<Image>>();
    assert_eq!(
        images.get(&Handle::<Image>::default()).unwrap().data,
        default_image.data
    );
    assert_eq!(images.get(&source).unwrap().data, default_image.data);
    assert_eq!(images.len(), 4);
}
