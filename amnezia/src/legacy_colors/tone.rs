use bevy::prelude::*;

pub(crate) fn uniform(percent: [f32; 4]) -> Vec4 {
    Vec4::from_array(percent.map(|value| (value * 128.0 / 100.0).trunc().clamp(0.0, 255.0)))
}

#[cfg(test)]
pub(crate) fn apply(source: [u8; 3], percent: [f32; 4]) -> [u8; 3] {
    let tone = uniform(percent).to_array().map(|v| v as i32);
    let mut rgb = source.map(i32::from);
    if tone[3] != 128 {
        let lum = (19595 * rgb[0] + 38470 * rgb[1] + 7471 * rgb[2]) >> 16;
        let sat = if tone[3] > 128 {
            1024 + (tone[3] - 128) * 16
        } else {
            tone[3] * 8
        };
        rgb = rgb.map(|c| ((lum * 1024 + (c - lum) * sat) >> 10).clamp(0, 255));
    }
    std::array::from_fn(|i| {
        let color = rgb[i];
        let level = tone[i];
        if level <= 128 {
            (2 * level * color / 255).clamp(0, 255) as u8
        } else {
            (255 - 2 * (255 - level) * (255 - color) / 255) as u8
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_channels_quantize_at_128_and_saturate_at_255() {
        assert_eq!(uniform([100.0; 4]), Vec4::splat(128.0));
        assert_eq!(
            uniform([50.0, 99.9, 150.0, 200.0]),
            Vec4::new(64.0, 127.0, 192.0, 255.0)
        );
        assert_eq!(
            uniform([-10.0, 300.0, 60.0, 0.0]),
            Vec4::new(0.0, 255.0, 76.0, 0.0)
        );
    }

    #[test]
    fn hard_light_saturation_order_and_integer_luminance_match_reference() {
        assert_eq!(
            apply([32, 156, 0], [50.0, 100.0, 150.0, 0.0]),
            [50, 101, 179]
        );
        assert_eq!(
            apply([32, 156, 0], [100.0, 100.0, 100.0, 150.0]),
            [0, 211, 0]
        );
        assert_eq!(
            apply([32, 156, 0], [150.0, 150.0, 150.0, 100.0]),
            [145, 207, 129]
        );
        for value in 0..=255 {
            assert_eq!(apply([value; 3], [100.0; 4]), [value; 3]);
            assert_eq!(apply([value; 3], [200.0, 200.0, 200.0, 100.0]), [255; 3]);
        }
    }
}
