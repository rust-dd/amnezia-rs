use super::*;

#[test]
fn list_background_matches_pixmans_fixed_point_boundary_samples() {
    for (position, expected) in [
        (0, 0),
        (6, 0),
        (7, 1),
        (19, 2),
        (45, 6),
        (201, 30),
        (207, 31),
    ] {
        assert_eq!(sample(position, 208), expected);
    }
    for size in [16, 32, 64, 136, 184, 208, 240, 320] {
        for position in 0..size {
            assert!(sample(position, size) < 32);
        }
    }
}

#[test]
fn an_unscaled_background_keeps_every_source_pixel() {
    for pixel in 0..32 {
        assert_eq!(sample(pixel, 32), pixel);
    }
}

#[test]
fn the_rasterized_background_has_native_dimensions_and_original_colors() {
    let mut skin = Image::new_fill(
        Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    for y in 0..32 {
        for x in 0..32 {
            skin.set_color_at(x, y, Color::srgba_u8(x as u8, y as u8, 123, 255))
                .unwrap();
        }
    }
    let image = render(&skin, UVec2::new(320, 208));
    assert_eq!(image.size(), UVec2::new(320, 208));
    for (x, y, color) in [
        (0, 0, [0, 0, 123, 255]),
        (4, 45, [0, 6, 123, 255]),
        (319, 207, [31, 31, 123, 255]),
    ] {
        assert_eq!(
            image.get_color_at(x, y).unwrap().to_srgba().to_u8_array(),
            color
        );
    }
}
