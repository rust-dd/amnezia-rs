use super::*;

#[test]
fn background_operations_keep_position_scroll_and_landing_distinct() {
    let map = MapData::for_test(40, 30);
    let half = Vec2::new(160.0, 120.0);
    let mut camera = CameraPan::default();
    camera.update(&map, Vec2::new(0.0, -8.0), half, 0.0);
    camera.scroll_to(&map, Vec2::new(24.25, -11.5), half);
    camera.round_jump(&map, half);
    camera.scroll_to(&map, Vec2::new(33.0, -8.0), half);
    assert_eq!(
        camera.take_background_scroll(),
        vec![
            BackgroundScroll {
                map_id: map.map_id,
                display: [168.0, 128.0],
                delta: None
            },
            BackgroundScroll {
                map_id: map.map_id,
                display: [184.25, 131.5],
                delta: Some([16.25, 3.5])
            },
            BackgroundScroll {
                map_id: map.map_id,
                display: [193.0, 128.0],
                delta: Some([1.0, -0.0])
            },
        ]
    );
    assert!(camera.take_background_scroll().is_empty());
}

#[test]
fn same_map_relocation_retains_earlier_scroll_and_cross_map_relocation_discards_it() {
    let mut map = MapData::for_test(40, 30);
    let half = Vec2::new(160.0, 120.0);
    let mut camera = CameraPan::default();
    camera.update(&map, Vec2::ZERO, half, 0.0);
    camera.scroll_to(&map, Vec2::new(9.0, 0.0), half);
    camera.recenter(false);
    camera.update(&map, Vec2::new(80.0, -64.0), half, 0.0);
    let operations = camera.take_background_scroll();
    assert_eq!(operations.len(), 3);
    assert_eq!(operations[1].delta, Some([1.0, -0.0]));
    assert_eq!(operations[2].delta, None);
    assert_eq!(operations[2].display, [248.0, 184.0]);
    camera.scroll_to(&map, Vec2::new(89.0, -64.0), half);
    camera.recenter(true);
    map.map_id += 1;
    camera.update(&map, Vec2::ZERO, half, 0.0);
    let operations = camera.take_background_scroll();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].map_id, map.map_id);
    assert_eq!(operations[0].delta, None);
}

#[test]
fn saved_pending_background_operations_resume_once_and_reject_nonfinite_coordinates() {
    let map = MapData::for_test(40, 30);
    let half = Vec2::new(160.0, 120.0);
    let mut camera = CameraPan::default();
    camera.update(&map, Vec2::ZERO, half, 0.0);
    camera.scroll_to(&map, Vec2::new(24.25, -11.5), half);
    let saved = camera.snapshot();
    let text = ron::to_string(&saved).unwrap();
    let mut restored = ron::from_str::<super::super::saved::CameraState>(&text)
        .unwrap()
        .into_pan();
    assert_eq!(
        restored.take_background_scroll(),
        camera.take_background_scroll()
    );
    assert!(restored.take_background_scroll().is_empty());
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for axis in 0..2 {
            for delta in [false, true] {
                let mut invalid_state = saved.clone();
                let operation = &mut invalid_state.background_scroll[1];
                if delta {
                    operation.delta.as_mut().unwrap()[axis] = invalid;
                } else {
                    operation.display[axis] = invalid;
                }
                assert!(!invalid_state.valid());
            }
        }
    }
}
