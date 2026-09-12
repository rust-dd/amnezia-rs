use super::*;

#[test]
fn the_streak_has_twenty_four_single_white_pixels_in_six_columns() {
    let expected = [
        5, 5, 5, 5, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1, 0, 0, 0, 0,
    ];
    for ((x, y), expected_x) in streak().into_iter().zip(expected) {
        assert_eq!(x, expected_x);
        assert!((0..24).contains(&y));
    }
    let pixels = surface(
        &[Drop {
            x: 10,
            y: 20,
            life: 12,
        }],
        0,
    );
    assert_eq!(pixels.iter().filter(|&&alpha| alpha != 0).count(), 24);
    for y in 0..24 {
        for x in 0..6 {
            assert_eq!(
                pixels[((y + 20) * WIDTH + x + 10) as usize],
                if x == expected[y as usize] { 60 } else { 0 }
            );
        }
    }
}

#[test]
fn strength_selects_twenty_sixty_or_hundred_drops_and_their_byte_alpha() {
    let drops = (0..100)
        .map(|index| Drop {
            x: index % 20 * 16,
            y: index / 20 * 26,
            life: 12,
        })
        .collect::<Vec<_>>();
    for (strength, count, expected) in [(0, 20, 60), (1, 60, 72), (2, 100, 84)] {
        let pixels = surface(&drops, strength);
        assert_eq!(
            pixels.iter().filter(|&&a| a == expected).count(),
            count * 24
        );
        assert!(pixels.iter().all(|&a| a == 0 || a == expected));
    }
    for life in 0..40 {
        let pixels = surface(&[Drop { x: 20, y: 20, life }], 2);
        assert_eq!(
            pixels[20 * WIDTH as usize + 25],
            if life <= 12 { life * 7 } else { 0 }
        );
    }
}

#[test]
fn overflow_copies_once_but_negative_coordinates_clip() {
    let pixels = surface(
        &[Drop {
            x: 319,
            y: 159,
            life: 12,
        }],
        0,
    );
    assert_eq!(pixels.iter().filter(|&&a| a != 0).count(), 24);
    for (x, y) in streak() {
        assert_eq!(
            pixels[(((159 + y) % HEIGHT) * WIDTH + (319 + x) % WIDTH) as usize],
            60
        );
    }
    for (drop, count) in [
        (
            Drop {
                x: -3,
                y: 20,
                life: 12,
            },
            12,
        ),
        (
            Drop {
                x: 40,
                y: 310,
                life: 1,
            },
            10,
        ),
    ] {
        assert_eq!(
            surface(&[drop], 0).iter().filter(|&&a| a != 0).count(),
            count
        );
    }
}

#[test]
fn the_viewport_repeats_every_one_hundred_sixty_rows_with_pan_and_shake_offsets() {
    let mut source = vec![0; (WIDTH * HEIGHT) as usize];
    source[10 * WIDTH as usize + 20] = 84;
    let mut pixels = vec![0; 320 * 240 * 4];
    viewport(&source, [3, -5], [128, 192, 255], &mut pixels);
    for y in [15, 175] {
        let start = (y * 320 + 17) * 4;
        assert_eq!(&pixels[start..start + 4], &[128, 192, 255, 84]);
    }
    assert_eq!(pixels.chunks_exact(4).filter(|p| p[3] != 0).count(), 2);
}
