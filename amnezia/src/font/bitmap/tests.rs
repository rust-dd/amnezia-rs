use super::*;

fn system() -> Image {
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
    for y in 0..80 {
        for x in 0..160 {
            pixel(&mut image, IVec2::new(x, y), [x as u8, y as u8, 70, 255]);
        }
    }
    image
}

#[test]
fn bitmap_letters_sample_the_original_palette_and_offset_shadow_at_native_pixels() {
    let font = BitmapFont::from_id(0);
    let text = PixelText {
        size: UVec2::new(10, 16),
        runs: vec![Run::new("A", 1, 2, 18)],
    };
    let image = font.render(&text, &system());
    let rows = [0u16, 12, 12, 18, 18, 30, 18, 18, 18, 0, 0, 0];
    for (y, bits) in rows.iter().enumerate() {
        for x in 0..6 {
            if bits & (1 << x) != 0 {
                assert_eq!(
                    image
                        .get_color_at(1 + x, 2 + y as u32)
                        .unwrap()
                        .to_srgba()
                        .to_u8_array(),
                    [130 + x as u8, 68 + y as u8, 70, 255]
                );
            }
        }
    }
    assert_eq!(
        image.get_color_at(6, 11).unwrap().to_srgba().to_u8_array(),
        [20, 40, 70, 255]
    );
    assert_eq!(image.get_color_at(0, 0).unwrap().alpha(), 0.0);
}

#[test]
fn both_embedded_faces_preserve_hungarian_accents_widths_and_clipping() {
    for id in [0, 1] {
        let font = BitmapFont::from_id(id);
        assert_eq!(font.width("ŐőŰű 123"), 48);
        assert_eq!(
            font.glyph('Ő').1,
            [36, 18, 12, 18, 18, 18, 18, 18, 12, 0, 0, 0]
        );
        let text = PixelText {
            size: UVec2::new(6, 12),
            runs: vec![Run::new("Ő\nŰ", -2, -1, 0)],
        };
        let image = font.render(&text, &system());
        assert_eq!(image.data.as_ref().unwrap().len(), 6 * 12 * 4);
    }
}

#[test]
fn changing_text_reuses_its_image_and_palette_changes_invalidate_the_cache() {
    use bevy::ecs::system::RunSystemOnce;
    let mut world = World::new();
    world.init_resource::<Assets<Image>>();
    let system = world.resource_mut::<Assets<Image>>().add(system());
    world.insert_resource(BitmapFont::from_id(0));
    world.insert_resource(Palette {
        source: system.clone(),
        revision: 0,
    });
    world.init_resource::<Messages<AssetEvent<Image>>>();
    let entity = world
        .spawn(PixelText {
            size: UVec2::new(20, 16),
            runs: vec![Run::new("A", 0, 0, 0)],
        })
        .id();
    world.run_system_once(rasterize).unwrap();
    let first = world.get::<ImageNode>(entity).unwrap().image.clone();
    world.get_mut::<PixelText>(entity).unwrap().runs[0].text = "B".into();
    world.run_system_once(rasterize).unwrap();
    assert_eq!(world.get::<ImageNode>(entity).unwrap().image, first);
    let before = world
        .resource::<Assets<Image>>()
        .get(&first)
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
    assert_ne!(
        world.resource::<Assets<Image>>().get(&first).unwrap().data,
        before
    );
    assert_eq!(world.resource::<Assets<Image>>().len(), 2);
}
