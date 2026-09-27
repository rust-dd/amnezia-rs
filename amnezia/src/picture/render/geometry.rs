use bevy::prelude::*;

pub(super) fn quad(center: Vec2, size: Vec2, zoom: f32, angle: f32) -> (Vec2, Vec2) {
    let center = center.trunc();
    let origin = (size / 2.0).floor();
    if angle != 0.0 {
        let offset = Mat2::from_angle(angle) * ((size / 2.0 - origin) * zoom);
        (center + offset, size * zoom)
    } else {
        let drawn = (size * zoom).floor();
        let top_left = center - (origin * zoom).floor();
        (top_left + drawn / 2.0, drawn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_picture_origins_match_the_original_integer_half_size_at_every_zoom() {
        for (size, zoom, expected_center, expected_size) in [
            (
                Vec2::splat(3.0),
                1.0,
                Vec2::new(72.5, 48.5),
                Vec2::splat(3.0),
            ),
            (
                Vec2::splat(3.0),
                8.0,
                Vec2::new(76.0, 52.0),
                Vec2::splat(24.0),
            ),
            (
                Vec2::new(5.0, 7.0),
                0.5,
                Vec2::new(72.0, 48.5),
                Vec2::new(2.0, 3.0),
            ),
            (
                Vec2::new(4.0, 8.0),
                1.2,
                Vec2::new(72.0, 48.5),
                Vec2::new(4.0, 9.0),
            ),
            (
                Vec2::new(4.0, 8.0),
                1.0,
                Vec2::new(72.0, 48.0),
                Vec2::new(4.0, 8.0),
            ),
        ] {
            assert_eq!(
                quad(Vec2::new(72.9, 48.2), size, zoom, 0.0),
                (expected_center, expected_size)
            );
        }
    }

    #[test]
    fn negative_positions_truncate_before_zoomed_integer_origins_are_subtracted() {
        assert_eq!(
            quad(Vec2::new(-2.9, -4.2), Vec2::splat(3.0), 1.5, 0.0),
            (Vec2::new(-1.0, -3.0), Vec2::splat(4.0))
        );
    }

    #[test]
    fn rotations_keep_the_integer_origin_and_clockwise_screen_orientation() {
        let center = Vec2::new(72.0, 48.0);
        for (angle, offset) in [
            (std::f32::consts::FRAC_PI_2, Vec2::new(-1.0, 1.0)),
            (std::f32::consts::PI, Vec2::new(-1.0, -1.0)),
            (std::f32::consts::PI * 1.5, Vec2::new(1.0, -1.0)),
        ] {
            let (actual, size) = quad(center, Vec2::splat(3.0), 2.0, angle);
            assert!(actual.abs_diff_eq(center + offset, 0.0001));
            assert_eq!(size, Vec2::splat(6.0));
        }
    }
}
