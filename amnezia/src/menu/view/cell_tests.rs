use crate::font::bitmap::{BitmapFont, PixelText, Run};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

pub(super) fn assert_row(actual: &PixelText, left: Vec<Run>, right: Vec<Run>, clear_next: bool) {
    let palette = Image::new_fill(
        Extent3d {
            width: 160,
            height: 80,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255; 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    for face in [0, 1] {
        let font = BitmapFont::from_id(face);
        let mut expected = font.render(
            &PixelText {
                size: actual.size,
                runs: left.clone(),
            },
            &palette,
        );
        let pixels = expected.data.as_mut().unwrap();
        assert!((2..14).any(|y| {
            pixels[(y * 304 + 160) * 4..(y * 304 + 304) * 4]
                .iter()
                .any(|v| *v != 0)
        }));
        if clear_next {
            for y in 2..14 {
                pixels[(y * 304 + 160) * 4..(y * 304 + 304) * 4].fill(0);
            }
        }
        if face == 1 {
            assert!(
                pixels[(14 * 304 + 160) * 4..15 * 304 * 4]
                    .iter()
                    .any(|v| *v != 0)
            );
        }
        let right = font.render(
            &PixelText {
                size: actual.size,
                runs: right.clone(),
            },
            &palette,
        );
        for (target, source) in pixels
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(right.data.unwrap().as_chunks::<4>().0)
        {
            if source[3] != 0 {
                target.copy_from_slice(source);
            }
        }
        let actual = font.render(actual, &palette);
        for (index, (actual, expected)) in actual
            .data
            .unwrap()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(pixels.as_chunks::<4>().0)
            .enumerate()
        {
            assert_eq!(
                actual,
                expected,
                "font {face}, pixel ({},{})",
                index % 304,
                index / 304
            );
        }
    }
}
