use super::*;

fn skin() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: 64,
            height: 32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    for y in 0..32 {
        for x in 0..64 {
            image
                .set_color_at(x, y, Color::srgba_u8(x as u8, y as u8, 123, 255))
                .unwrap();
        }
    }
    image
}

#[test]
fn every_animation_height_crops_the_full_background_and_moves_only_the_borders() {
    let skin = skin();
    for size in [UVec2::new(320, 80), UVec2::new(88, 32)] {
        let full = render(
            &skin,
            Pixels {
                size,
                half: size.y / 2,
            },
        );
        for half in 0..=size.y / 2 {
            let image = render(&skin, Pixels { size, half });
            let top = size.y / 2 - half;
            let bottom = size.y / 2 + half;
            for y in 0..size.y {
                for x in 0..size.x {
                    let expected = if y < top || y >= bottom {
                        [0; 4]
                    } else if y < top + half.min(8) {
                        rgba(&full, x, y - top)
                    } else if y >= bottom - half.min(8) {
                        rgba(&full, x, size.y - bottom + y)
                    } else {
                        rgba(&full, x, y)
                    };
                    assert_eq!(rgba(&image, x, y), expected, "{size:?}, {half}, {x}, {y}");
                }
            }
        }
    }
}

#[test]
fn border_tiles_keep_the_original_source_offsets_while_the_background_stays_fixed() {
    let skin = skin();
    let image = render(
        &skin,
        Pixels {
            size: UVec2::new(320, 80),
            half: 22,
        },
    );
    for (x, y, sx, sy) in [
        (0, 18, 32, 0),
        (319, 18, 63, 0),
        (8, 18, 48, 0),
        (16, 18, 40, 0),
        (0, 26, 32, 18),
        (0, 34, 32, 10),
        (0, 61, 32, 31),
        (319, 61, 63, 31),
        (8, 61, 48, 31),
        (160, 40, 16, 16),
    ] {
        assert_eq!(rgba(&image, x, y), rgba(&skin, sx, sy), "{x}, {y}");
    }
    let image = render(
        &skin,
        Pixels {
            size: UVec2::new(320, 80),
            half: 5,
        },
    );
    for (x, y, sx, sy) in [
        (0, 35, 32, 0),
        (0, 39, 32, 4),
        (0, 40, 32, 27),
        (0, 44, 32, 31),
    ] {
        assert_eq!(rgba(&image, x, y), rgba(&skin, sx, sy));
    }
}

#[test]
fn transparent_border_pixels_expose_the_original_full_height_background() {
    let mut skin = skin();
    skin.set_color_at(32, 0, Color::NONE).unwrap();
    let image = render(
        &skin,
        Pixels {
            size: UVec2::new(320, 80),
            half: 5,
        },
    );
    assert_eq!(rgba(&image, 0, 35), [0, 14, 123, 255]);
}
