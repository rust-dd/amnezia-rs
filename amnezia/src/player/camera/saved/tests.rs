use super::*;
use crate::world::MapData;

#[test]
fn serialized_camera_resumes_pan_and_loop_tracking_without_a_follow_jump() {
    for fps in [15, 30, 60, 144] {
        for locked in [false, true] {
            let mut map = MapData::for_test(140, 140);
            map.scroll_type = 3;
            let mut original = CameraPan::default();
            let point = |frame| {
                Vec2::new(
                    (1112.0_f32 + frame as f32 * 2.0 + 1120.0).rem_euclid(2240.0) - 1120.0,
                    0.0,
                )
            };
            original.update(&map, point(0), Vec2::new(160.0, 120.0), 0.0);
            original.locked = locked;
            original.command(&[2, 0, 20, 1, 0]);
            for frame in 1..=30 {
                original.update(
                    &map,
                    point(frame),
                    Vec2::new(160.0, 120.0),
                    1.0 / fps as f32,
                );
            }
            let state = original.snapshot();
            assert!(state.valid());
            let text = ron::to_string(&state).unwrap();
            let mut restored = ron::from_str::<CameraState>(&text).unwrap().into_pan();
            for frame in 31..=130 {
                assert_eq!(
                    restored.update(
                        &map,
                        point(frame),
                        Vec2::new(160.0, 120.0),
                        1.0 / fps as f32
                    ),
                    original.update(
                        &map,
                        point(frame),
                        Vec2::new(160.0, 120.0),
                        1.0 / fps as f32
                    )
                );
                assert_eq!(restored.snapshot(), original.snapshot());
            }
        }
    }
}

#[test]
fn invalid_camera_numbers_cannot_enter_a_loaded_scene() {
    let original = CameraPan::default().snapshot();
    assert!(original.valid());
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for field in 0..5 {
            let mut state = original.clone();
            match field {
                0 => state.offset[0] = invalid,
                1 => state.target[1] = invalid,
                2 => state.speed = invalid,
                3 => state.position = Some([invalid, 0.0]),
                _ => state.previous_player = Some([0.0, invalid]),
            }
            assert!(!state.valid());
        }
    }
    for speed in [0.0, -1.0] {
        assert!(
            !CameraState {
                speed,
                ..original.clone()
            }
            .valid()
        );
    }
}

#[test]
fn clearing_the_session_cancels_a_waiting_camera_restore() {
    let mut world = World::new();
    prepare(&mut world, 13, Some(CameraPan::default().snapshot()));
    assert!(world.contains_resource::<Pending>());
    crate::session::clear_transient(&mut world);
    assert!(!world.contains_resource::<Pending>());
    assert_eq!(
        world.resource::<CameraPan>().snapshot(),
        CameraPan::default().snapshot()
    );
}

#[test]
fn legacy_camera_snapshots_start_effect_tracking_without_rewriting_their_position() {
    let text = "(offset:(8.25,0.0),target:(16.0,0.0),speed:15.0,locked:true,position:Some((24.25,-8.0)),previous_player:Some((8.0,-8.0)))";
    let saved = ron::from_str::<CameraState>(text).unwrap();
    assert!(saved.valid());
    assert!(saved.tracking.is_none());
    let mut pan = saved.into_pan();
    assert_eq!(pan.effects_position(), Some(Vec2::new(24.25, -8.0)));
    pan.update(
        &MapData::for_test(40, 30),
        Vec2::new(8.0, -8.0),
        Vec2::new(160.0, 120.0),
        1.0 / 60.0,
    );
    assert_eq!(pan.position, Some(Vec2::new(24.5, -8.0)));
    assert_eq!(pan.effects_position(), pan.position);
    assert!(pan.snapshot().valid());
}
