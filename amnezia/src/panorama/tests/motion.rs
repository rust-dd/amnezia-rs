use super::*;
use crate::panorama::motion::{Motion, amount};
use crate::player::BackgroundScroll;

fn definition(params: &[i32]) -> PanoramaDef {
    PanoramaDef::from_command("Sky".into(), params)
}

fn event(map: &MapData, display: [f32; 2], delta: Option<[f32; 2]>) -> BackgroundScroll {
    BackgroundScroll {
        map_id: map.map_id,
        display,
        delta,
    }
}

#[test]
fn initialization_distinguishes_background_loops_from_map_loops_and_bounded_axes() {
    for map_loops in 0..4 {
        for x_loop in [false, true] {
            for y_loop in [false, true] {
                let mut map = MapData::for_test(40, 30);
                map.scroll_type = map_loops;
                let def = definition(&[x_loop as i32, y_loop as i32, 0, 0, 0, 0]);
                let mut state = Motion::default();
                state.initialize(
                    "Sky",
                    UVec2::new(640, 480),
                    Some(&def),
                    &map,
                    Vec2::new(178.25, 128.5),
                );
                assert_eq!(
                    state.phase,
                    [
                        if x_loop || map.loops_x() { 2852 } else { 5704 },
                        if y_loop || map.loops_y() { 2056 } else { 4112 },
                    ]
                );
            }
        }
    }
}

#[test]
fn bounded_ratios_truncate_subpixels_before_source_pixels_and_cap_large_images() {
    let map = MapData::for_test(45, 33);
    let def = definition(&[0; 6]);
    let mut state = Motion::default();
    state.initialize(
        "Sky",
        UVec2::new(640, 481),
        Some(&def),
        &map,
        Vec2::new(137.0625, 57.9375),
    );
    assert_eq!(state.phase, [3508, 1551]);
    assert_eq!(state.offset(), Vec2::new(531.0, 433.0));
    state.initialize(
        "Large",
        UVec2::new(1200, 900),
        Some(&def),
        &map,
        Vec2::new(137.0625, 57.9375),
    );
    assert_eq!(state.phase, [4386, 1854]);
}

#[test]
fn same_image_flags_keep_phase_until_actual_axis_scroll_or_explicit_positioning() {
    let map = MapData::for_test(40, 30);
    let scrolling = definition(&[1, 1, 0, 0, 0, 0]);
    let bounded = definition(&[0; 6]);
    let mut state = Motion::default();
    state.initialize(
        "Sky",
        UVec2::new(640, 480),
        Some(&scrolling),
        &map,
        Vec2::new(160.0, 120.0),
    );
    state.initialize(
        "Sky",
        UVec2::new(640, 480),
        Some(&bounded),
        &map,
        Vec2::new(180.0, 144.0),
    );
    assert_eq!(state.phase, [2560, 1920]);
    state.scroll(
        &event(&map, [181.0, 144.0], Some([1.0, 0.0])),
        Some(&bounded),
        &map,
    );
    assert_eq!(state.phase, [5792, 1920]);
    state.scroll(&event(&map, [128.0, 80.0], None), Some(&scrolling), &map);
    assert_eq!(state.phase, [2048, 1280]);
    state.scroll(
        &event(&map, [132.0, 96.0], Some([4.0, 16.0])),
        Some(&scrolling),
        &map,
    );
    assert_eq!(state.phase, [2112, 1536]);
}

#[test]
fn a_nonlooping_background_on_a_looping_map_ignores_scroll_but_not_relocation() {
    let mut map = MapData::for_test(40, 30);
    map.scroll_type = 3;
    let def = definition(&[0; 6]);
    let mut state = Motion::default();
    state.initialize(
        "Sky",
        UVec2::new(640, 480),
        Some(&def),
        &map,
        Vec2::new(160.0, 120.0),
    );
    state.scroll(
        &event(&map, [176.0, 104.0], Some([16.0, -16.0])),
        Some(&def),
        &map,
    );
    assert_eq!(state.phase, [2560, 1920]);
    state.scroll(&event(&map, [32.0, 64.0], None), Some(&def), &map);
    assert_eq!(state.phase, [512, 1024]);
}

#[test]
fn replacing_a_looping_bitmap_keeps_phase_modulo_its_new_dimensions() {
    let map = MapData::for_test(40, 30);
    let def = definition(&[1, 1, 1, 1, 1, -1]);
    let mut state = Motion::default();
    state.initialize(
        "Sky",
        UVec2::new(640, 480),
        Some(&def),
        &map,
        Vec2::new(160.0, 120.0),
    );
    state.step(&def, 3);
    assert_eq!(state.phase, [2554, 1926]);
    state.initialize(
        "Ground",
        UVec2::new(16, 16),
        Some(&def),
        &map,
        Vec2::new(190.0, 150.0),
    );
    assert_eq!(state.phase, [506, 390]);
    state.initialize(
        "Morning1",
        UVec2::new(640, 480),
        Some(&def),
        &map,
        Vec2::new(190.0, 150.0),
    );
    assert_eq!(state.phase, [506, 390]);
    state.initialize(
        "Night1",
        UVec2::new(640, 480),
        Some(&definition(&[0; 6])),
        &map,
        Vec2::new(190.0, 150.0),
    );
    assert_eq!(state.phase, [6080, 4800]);
}

#[test]
fn signed_remainders_and_division_match_the_original_integer_raster() {
    let map = MapData::for_test(40, 30);
    let def = definition(&[1, 1, 1, 0, 1, 0]);
    let mut state = Motion::default();
    state.initialize("Sky", UVec2::new(640, 480), Some(&def), &map, Vec2::ZERO);
    state.scroll(
        &event(&map, [-1281.0625, 0.0], Some([-1281.0625, 0.0])),
        Some(&def),
        &map,
    );
    assert_eq!(state.phase, [-17, 0]);
    state.scroll(
        &event(&map, [-1281.0625, 1.0], Some([0.0, 1.0])),
        Some(&def),
        &map,
    );
    state.step(&def, 100);
    assert_eq!(state.phase, [-17, 16]);
    for (phase, offset) in [
        (-33, 1.0),
        (-32, 1.0),
        (-31, 0.0),
        (-1, 0.0),
        (0, 0.0),
        (31, 0.0),
        (32, 639.0),
        (33, 639.0),
    ] {
        state.phase[0] = phase;
        assert_eq!(state.offset().x, offset);
    }
    assert_eq!(amount(0), 0);
    assert_eq!(amount(8), -256);
    assert_eq!(amount(-6), 64);
}

#[test]
fn phase_validation_rejects_invalid_saved_periods_and_overflows() {
    let mut state = Motion::default();
    assert!(state.valid());
    state.size = [640, 480];
    for phase in [i64::MIN, i64::MAX, 640 * 32, -640 * 32] {
        state.phase = [phase, 0];
        assert!(!state.valid());
    }
    state.phase = [0; 2];
    state.size = [u32::MAX, 480];
    assert!(!state.valid());
}
