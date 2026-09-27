use super::*;
use crate::world::EventSprite;

fn character(tile: (i32, i32)) -> EventSprite {
    EventSprite {
        id: 1,
        tile_x: tile.0,
        tile_y: tile.1,
        dir: DIR_DOWN,
        frame: 1,
        charset: String::new(),
        index: 0,
        layer: 1,
    }
}

#[test]
fn loop_seams_rasterize_canonical_coordinates_before_screen_wrapping() {
    let mut data = MapData::for_test(20, 15);
    data.scroll_type = 3;
    for (origin, dir) in [
        ((0, 0), DIR_LEFT),
        ((0, 0), DIR_UP),
        ((19, 14), DIR_RIGHT),
        ((19, 14), DIR_DOWN),
    ] {
        for jumping in [false, true] {
            let mut ch = character(origin);
            let mut queue = MoveQueue::default();
            queue.use_character_motion(1, dir);
            let (dx, dy) = dir_delta(dir);
            queue.push_step(if jumping {
                RouteAction::Jump { dx, dy, face: dir }
            } else {
                RouteAction::Step { dx, dy, face: dir }
            });
            let amount = if jumping { 8 } else { 4 };
            let mut remaining = 256;
            while remaining > 0 {
                queue.advance(&mut ch, &data, 1.0 / 60.0);
                remaining = (remaining - amount).max(0);
                let map_x = (ch.tile_x * 256 - dx * remaining) / 16;
                let map_y = (ch.tile_y * 256 - dy * remaining) / 16;
                let canonical = center(&data, 0, 0) + Vec2::new(map_x as f32, -map_y as f32);
                let raw = queue.subpixel_position(&ch, &data);
                assert_eq!(
                    queue.ground_position(&ch, &data),
                    data.world_near(canonical, raw),
                    "origin {origin:?}, direction {dir}, jump {jumping}, remaining {remaining}"
                );
            }
        }
    }
}

#[test]
fn negative_unwrapped_positions_use_integer_division_toward_zero() {
    let data = MapData::for_test(20, 15);
    for dir in [DIR_LEFT, DIR_UP] {
        let mut ch = character((0, 0));
        let mut queue = MoveQueue::default();
        queue.use_character_motion(1, dir);
        let (dx, dy) = dir_delta(dir);
        queue.push_step(RouteAction::Step { dx, dy, face: dir });
        for frame in 1..=5 {
            queue.advance(&mut ch, &data, 1.0 / 60.0);
            let pixels = (frame / 4) as f32;
            assert_eq!(
                queue.ground_position(&ch, &data),
                center(&data, 0, 0) + Vec2::new(dx as f32, -dy as f32) * pixels
            );
        }
    }
}

#[test]
fn subpixel_motion_keeps_logical_direction_separate_from_a_locked_sprite_pose() {
    let data = MapData::for_test(20, 15);
    for dir in 0..8 {
        let mut ch = character((5, 5));
        let mut queue = MoveQueue::default();
        queue.use_character_motion(4, dir);
        let (dx, dy) = dir_delta(dir);
        queue.push_step(RouteAction::Step {
            dx,
            dy,
            face: DIR_DOWN,
        });
        queue.advance(&mut ch, &data, 1.0 / 60.0);
        assert_eq!(ch.dir, DIR_DOWN);
        assert_eq!(
            queue.ground_position(&ch, &data),
            center(&data, 5, 5) + Vec2::new(dx as f32, -dy as f32) * 2.0
        );
    }
}

#[test]
fn live_jump_speed_changes_preserve_the_remaining_subpixels() {
    let data = MapData::for_test(20, 15);
    let mut ch = character((5, 5));
    let mut queue = MoveQueue::default();
    queue.use_character_motion(4, DIR_RIGHT);
    queue.push_step(RouteAction::Jump {
        dx: 2,
        dy: 0,
        face: DIR_RIGHT,
    });
    queue.advance(&mut ch, &data, 1.0 / 60.0);
    queue.use_character_motion(6, DIR_UP);
    queue.advance(&mut ch, &data, 1.0 / 60.0);
    assert_eq!(
        queue.ground_position(&ch, &data),
        center(&data, 5, 5) + Vec2::X * 11.0
    );
    assert_eq!(
        queue.render_position(&ch, &data),
        center(&data, 5, 5) + Vec2::new(11.0, 15.0)
    );
    for _ in 0..3 {
        queue.advance(&mut ch, &data, 1.0 / 60.0);
    }
    assert!(!queue.busy());
    assert_eq!(ch.tile(), (7, 5));
}

#[test]
fn saved_partial_frame_motion_continues_exactly_at_different_render_rates() {
    let data = MapData::for_test(20, 15);
    for fps in [15, 30, 60, 144] {
        let mut ch = character((5, 5));
        let mut queue = MoveQueue::default();
        queue.use_character_motion(2, DIR_RIGHT);
        queue.push_step(RouteAction::Jump {
            dx: 2,
            dy: 1,
            face: DIR_RIGHT,
        });
        queue.advance(&mut ch, &data, 1.0 / 144.0);
        let encoded = ron::to_string(&queue.snapshot()).unwrap();
        let state = ron::from_str::<saved::MotionState>(&encoded).unwrap();
        assert!(state.valid_for_event());
        let mut restored = state.into_queue();
        let mut restored_character = ch.clone();
        for _ in 0..fps {
            assert_eq!(
                restored.advance(&mut restored_character, &data, 1.0 / fps as f32),
                queue.advance(&mut ch, &data, 1.0 / fps as f32)
            );
            assert_eq!(restored.snapshot(), queue.snapshot());
        }
        assert!(!queue.busy());
    }
}

#[test]
fn invalid_subpixel_snapshots_are_rejected() {
    let data = MapData::for_test(20, 15);
    let mut ch = character((5, 5));
    let mut queue = MoveQueue::default();
    queue.use_character_motion(4, DIR_RIGHT);
    queue.push_step(RouteAction::Step {
        dx: 1,
        dy: 0,
        face: DIR_RIGHT,
    });
    queue.advance(&mut ch, &data, 0.0);
    let encoded = ron::to_string(&queue.snapshot()).unwrap();
    for (from, to) in [
        ("remaining:256", "remaining:0"),
        ("remaining:256", "remaining:257"),
        ("fraction:0.0", "fraction:NaN"),
        ("fraction:0.0", "fraction:1.0"),
        ("speed:4", "speed:0"),
        ("speed:4", "speed:7"),
        ("direction:1", "direction:8"),
        ("kinematics:Some((speed:4,direction:1))", "kinematics:None"),
    ] {
        assert!(encoded.contains(from), "{from}: {encoded}");
        let state = ron::from_str::<saved::MotionState>(&encoded.replace(from, to)).unwrap();
        assert!(!state.valid_for_event(), "{from} -> {to}");
    }
}

#[test]
fn legacy_tweens_migrate_once_when_the_live_character_resumes() {
    let state = ron::from_str::<saved::MotionState>(
        "(steps:[],active:Some((from:(0.0,0.0),to:(16.0,0.0),elapsed:0.1,jumping:false)),step_secs:0.2)",
    ).unwrap();
    assert!(state.valid_for_event());
    let mut queue = state.into_queue();
    queue.use_character_motion(4, DIR_RIGHT);
    let original = queue.snapshot();
    queue.use_character_motion(4, DIR_RIGHT);
    assert_eq!(original, queue.snapshot());
    assert!(ron::to_string(&original).unwrap().contains("remaining:128"));
}
