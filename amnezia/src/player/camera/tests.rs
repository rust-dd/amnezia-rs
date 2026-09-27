use super::*;

mod walking;

const HALF_VIEW: Vec2 = Vec2::new(160.0, 120.0);

#[test]
fn camera_rasterization_truncates_map_display_coordinates_without_losing_pan_fraction() {
    let data = MapData::for_test(40, 30);
    let mut pan = CameraPan::default();
    pan.update(&data, Vec2::ZERO, HALF_VIEW, 0.0);
    pan.command(&[2, 1, 1, 1, 0]);
    for frame in 1..=8 {
        let raw = pan.update(&data, Vec2::ZERO, HALF_VIEW, 1.0 / 60.0);
        assert_eq!(raw.x, 8.0 + frame as f32 / 4.0);
        assert_eq!(
            raster_position(&data, raw, HALF_VIEW).x,
            8.0 + (frame / 4) as f32
        );
        assert_eq!(pan.offset.x, frame as f32 / 4.0);
    }
    let corner = Vec2::from(data.tile_center(0, 0)) + Vec2::new(-8.0, 8.0);
    let raw = corner + Vec2::new(HALF_VIEW.x - 0.25, -HALF_VIEW.y + 0.25);
    assert_eq!(
        raster_position(&data, raw, HALF_VIEW),
        raw + Vec2::new(0.25, -0.25)
    );
}

#[test]
fn original_long_cutscene_pans_keep_their_full_wait_even_at_the_map_edge() {
    for (map_id, expected_frames) in [(96, 400.0), (211, 1600.0), (222, 800.0), (275, 1600.0)] {
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/maps/map_{map_id:04}.ron",
            crate::assets::asset_root()
        ));
        let command = map
            .events
            .iter()
            .flat_map(|e| &e.pages)
            .flat_map(|p| &p.commands)
            .find(|c| c.code == 11060 && c.params[2] == 50)
            .unwrap();
        let mut pan = CameraPan::default();
        assert_eq!(pan.command(&command.params), expected_frames / 60.0);
    }
}

#[test]
fn original_pan_speeds_and_waits_are_exponential_and_return_uses_its_own_speed() {
    for speed in 1..=6 {
        let mut pan = CameraPan::default();
        let wait = pan.command(&[2, 0, 2, speed, 1]);
        assert_eq!(pan.target, Vec2::Y * 32.0);
        assert_eq!(pan.speed, (2u32 << speed) as f32 / 16.0 * 60.0);
        assert_eq!(wait, 512.0 / (2u32 << speed) as f32 / 60.0);
    }
    let mut pan = CameraPan {
        offset: Vec2::new(160.0, -320.0),
        target: Vec2::new(160.0, -320.0),
        ..default()
    };
    assert_eq!(pan.command(&[3, 0, 1, 6, 1]), 40.0 / 60.0);
    assert_eq!(pan.speed, 480.0);
    assert_eq!(pan.target, Vec2::ZERO);
}

#[test]
fn pan_axes_advance_independently_and_wait_covers_all_pending_pan_commands() {
    let mut pan = CameraPan::default();
    assert_eq!(pan.command(&[2, 1, 20, 3, 0]), 0.0);
    assert_eq!(pan.command(&[2, 0, 2, 3, 1]), 320.0 / 60.0);
    assert_eq!(ease_toward(Vec2::ZERO, pan.target, 10.0), Vec2::splat(10.0));
    let map = MapData::for_test(140, 140);
    assert_eq!(
        pan.update(&map, Vec2::ZERO, HALF_VIEW, 1.0),
        Vec2::new(68.0, 32.0)
    );
    assert_eq!(pan.offset, Vec2::new(60.0, 32.0));
}

#[test]
fn camera_lock_stops_following_but_not_panning_and_unlock_does_not_snap() {
    let map = MapData::for_test(140, 140);
    let mut pan = CameraPan::default();
    assert_eq!(
        pan.update(&map, Vec2::ZERO, HALF_VIEW, 0.0),
        Vec2::new(8.0, 0.0)
    );
    pan.command(&[0]);
    assert_eq!(
        pan.update(&map, Vec2::new(16.0, 0.0), HALF_VIEW, 0.0),
        Vec2::new(8.0, 0.0)
    );
    pan.command(&[2, 2, 2, 3, 0]);
    assert_eq!(
        pan.update(&map, Vec2::new(16.0, 0.0), HALF_VIEW, 1.0),
        Vec2::new(8.0, -32.0)
    );
    pan.command(&[1]);
    assert_eq!(
        pan.update(&map, Vec2::new(16.0, 0.0), HALF_VIEW, 0.0),
        Vec2::new(8.0, -32.0)
    );
    assert_eq!(
        pan.update(&map, Vec2::new(32.0, 0.0), HALF_VIEW, 0.0),
        Vec2::new(24.0, -32.0)
    );
}

#[test]
fn pan_stops_at_bounded_map_edges_without_consuming_the_untravelled_offset() {
    let map = MapData::for_test(40, 30);
    let mut pan = CameraPan::default();
    pan.update(&map, Vec2::ZERO, HALF_VIEW, 0.0);
    let wait = pan.command(&[2, 1, 50, 4, 1]);
    assert_eq!(wait, 400.0 / 60.0);
    assert_eq!(
        pan.update(&map, Vec2::ZERO, HALF_VIEW, 10.0),
        Vec2::new(160.0, 0.0)
    );
    assert_eq!(pan.offset.x, 152.0);
    assert_eq!(pan.target.x, 800.0);
    assert_eq!(pan.command(&[3, 0, 1, 3, 1]), 152.0 / 60.0);
    assert_eq!(
        pan.update(&map, Vec2::ZERO, HALF_VIEW, 10.0),
        Vec2::new(8.0, 0.0)
    );
}

#[test]
fn transfers_preserve_lock_and_only_cross_map_transfers_reset_pan_offsets() {
    let mut pan = CameraPan {
        locked: true,
        offset: Vec2::splat(32.0),
        target: Vec2::splat(64.0),
        position: Some(Vec2::ONE),
        ..default()
    };
    pan.recenter(false);
    assert!(pan.locked && pan.position.is_none());
    assert_eq!(pan.offset, Vec2::splat(32.0));
    assert_eq!(pan.target, Vec2::splat(64.0));
    pan.recenter(true);
    assert!(pan.locked);
    assert_eq!(pan.offset, Vec2::ZERO);
    assert_eq!(pan.target, Vec2::ZERO);
}

#[test]
fn following_is_continuous_across_loop_edges_and_pan_duration_is_frame_rate_independent() {
    let mut map = MapData::for_test(140, 140);
    map.scroll_type = 3;
    for fps in [30, 60, 144] {
        let mut pan = CameraPan::default();
        pan.update(&map, Vec2::new(-1112.0, 1112.0), HALF_VIEW, 0.0);
        let point = pan.update(&map, Vec2::new(1112.0, -1112.0), HALF_VIEW, 0.0);
        assert_eq!(point, Vec2::new(-1120.0, 1128.0));
        pan.command(&[2, 1, 10, 3, 0]);
        for _ in 0..fps {
            pan.update(
                &map,
                Vec2::new(1112.0, -1112.0),
                HALF_VIEW,
                1.0 / fps as f32,
            );
        }
        assert!((pan.offset.x - 60.0).abs() < 0.02);
    }
}
