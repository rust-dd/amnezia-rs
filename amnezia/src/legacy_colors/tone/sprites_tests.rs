use super::*;
use crate::legacy_colors::{
    flash::SpriteFlash,
    hue::{self, HueShift},
};
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[test]
fn sprite_tone_follows_hue_precedes_flash_and_preserves_source_alpha() {
    let mut app = App::new();
    app.init_resource::<Assets<Image>>();
    hue::register(&mut app);
    let image = Image::new(
        Extent3d {
            width: 3,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![32, 156, 0, 255, 32, 156, 0, 128, 12, 34, 56, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let source = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(image.clone());
    let plain = app
        .world_mut()
        .spawn((
            Sprite::from_image(source.clone()),
            HueShift {
                original: source.clone(),
                degrees: 0,
            },
            SpriteTone([50.0, 100.0, 150.0, 0.0]),
        ))
        .id();
    let flashing = app
        .world_mut()
        .spawn((
            Sprite::from_image(source.clone()),
            HueShift {
                original: source.clone(),
                degrees: 120,
            },
            SpriteTone([50.0, 100.0, 150.0, 0.0]),
            SpriteFlash([248, 160, 80, 96]),
        ))
        .id();
    app.update();
    let pixels = |app: &App, entity| {
        let sprite = app.world().get::<Sprite>(entity).unwrap();
        app.world()
            .resource::<Assets<Image>>()
            .get(&sprite.image)
            .unwrap()
            .data
            .clone()
            .unwrap()
    };
    assert_eq!(
        pixels(&app, plain),
        [50, 101, 179, 255, 50, 101, 179, 128, 12, 34, 56, 0]
    );
    assert_eq!(
        pixels(&app, flashing),
        [104, 82, 122, 255, 104, 82, 122, 128, 12, 34, 56, 0]
    );
    let private = app.world().get::<Sprite>(flashing).unwrap().image.clone();
    app.world_mut().get_mut::<SpriteTone>(flashing).unwrap().0 = [200.0, 200.0, 200.0, 100.0];
    app.update();
    assert_eq!(app.world().get::<Sprite>(flashing).unwrap().image, private);
    assert_eq!(&pixels(&app, flashing)[..4], &[252, 219, 189, 255]);
    app.world_mut().get_mut::<SpriteTone>(plain).unwrap().0 = [100.0; 4];
    app.world_mut().get_mut::<SpriteTone>(flashing).unwrap().0 = [100.0; 4];
    app.world_mut().get_mut::<SpriteFlash>(flashing).unwrap().0 = [0; 4];
    app.update();
    assert_eq!(app.world().get::<Sprite>(plain).unwrap().image, source);
    assert_ne!(app.world().get::<Sprite>(flashing).unwrap().image, private);
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&source)
            .unwrap()
            .data,
        image.data
    );
    assert_eq!(app.world().resource::<Assets<Image>>().len(), 4);
}
