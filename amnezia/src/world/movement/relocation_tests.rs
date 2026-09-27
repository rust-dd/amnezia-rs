use super::*;
use crate::world::EventSprite;

fn character(tile: (i32, i32)) -> EventSprite {
    EventSprite {
        id: 1,
        tile_x: tile.0,
        tile_y: tile.1,
        dir: DIR_RIGHT,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
        layer: 1,
    }
}

#[test]
fn relocation_preserves_a_jumps_canonical_origin_across_map_sizes_and_loop_seams() {
    for (speed, amount) in [(1, 8), (2, 12), (3, 16), (4, 24), (5, 32), (6, 64)] {
        for legacy in [false, true] {
            let mut old_map = MapData::for_test(20, 15);
            old_map.scroll_type = 3;
            let destination = MapData::for_test(40, 30);
            let mut ch = character((19, 14));
            let mut queue = MoveQueue::default();
            queue.use_character_motion(speed, DIR_RIGHT);
            queue.begin_from(
                &mut ch,
                &old_map,
                (19, 14),
                RouteAction::Jump {
                    dx: 2,
                    dy: 2,
                    face: DIR_RIGHT,
                },
            );
            queue.advance(&mut ch, &old_map, 1.0 / 60.0);
            assert_eq!(ch.tile(), (1, 1));
            if legacy {
                queue.active.as_mut().unwrap().jump_origin = None;
            }
            queue.relocate(ch.tile());
            ch.set_tile(8, 9);
            let point = center(&destination, 8, 9);
            assert!(queue.busy() && queue.jumping());
            assert_eq!(queue.render_position(&ch, &destination), point);
            let scroll = queue.scroll_step(ch.tile(), 1.0 / 60.0).unwrap();
            assert_eq!(scroll.pixels, amount as f32 / 16.0);
            assert_eq!(scroll.jump_delta, Some(Vec2::new(9.0, 10.0)));
            assert!(scroll.landing);
            let encoded = ron::to_string(&queue.snapshot()).unwrap();
            let state = ron::from_str::<saved::MotionState>(&encoded).unwrap();
            assert!(state.valid_for_event());
            let mut restored = state.into_queue();
            assert_eq!(restored.snapshot(), queue.snapshot());
            assert_eq!(
                restored.advance(&mut ch, &destination, 1.0 / 60.0),
                Some(point)
            );
            assert!(!restored.busy());
            assert_eq!(restored.render_position(&ch, &destination), point);
        }
    }
}

#[test]
fn relocation_keeps_detached_and_legacy_zero_remaining_jumps_serializable() {
    for legacy in [false, true] {
        let data = MapData::for_test(20, 15);
        let mut ch = character((2, 3));
        let mut queue = MoveQueue::default();
        queue.begin_from(
            &mut ch,
            &data,
            (2, 3),
            RouteAction::Jump {
                dx: 2,
                dy: 1,
                face: DIR_RIGHT,
            },
        );
        if legacy {
            queue.active.as_mut().unwrap().jump_origin = None;
        }
        queue.relocate(ch.tile());
        ch.set_tile(8, 9);
        assert!(queue.snapshot().valid());
        assert_eq!(queue.render_position(&ch, &data), center(&data, 8, 9));
        queue.use_character_motion(2, DIR_RIGHT);
        assert_eq!(
            queue.active.as_ref().unwrap().subpixels.unwrap().remaining,
            0
        );
        assert!(queue.snapshot().valid());
        assert_eq!(
            queue.scroll_step(ch.tile(), 1.0 / 60.0).unwrap().jump_delta,
            Some(Vec2::splat(6.0))
        );
        assert_eq!(
            queue.advance(&mut ch, &data, 1.0 / 60.0),
            Some(center(&data, 8, 9))
        );
    }
}

#[test]
fn relocation_clears_walks_and_pending_steps_without_changing_motion_parameters() {
    let data = MapData::for_test(20, 15);
    let mut ch = character((2, 3));
    let mut queue = MoveQueue::default();
    queue.set_step_secs(0.75);
    queue.use_character_motion(2, DIR_RIGHT);
    let step = RouteAction::Step {
        dx: 1,
        dy: 0,
        face: DIR_RIGHT,
    };
    queue.begin_from(&mut ch, &data, (2, 3), step.clone());
    queue.push_step(step);
    let motion = queue.kinematics;
    queue.relocate(ch.tile());
    assert!(!queue.busy());
    assert_eq!(queue.step_secs, 0.75);
    assert_eq!(queue.kinematics, motion);
    assert!(queue.snapshot().valid());
}

#[test]
fn relocation_snapshots_reject_zero_steps_without_a_finished_jump() {
    let data = MapData::for_test(20, 15);
    let mut ch = character((2, 3));
    let mut queue = MoveQueue::default();
    queue.use_character_motion(2, DIR_RIGHT);
    queue.begin_from(
        &mut ch,
        &data,
        (2, 3),
        RouteAction::Jump {
            dx: 2,
            dy: 0,
            face: DIR_RIGHT,
        },
    );
    queue.relocate(ch.tile());
    assert!(queue.snapshot().valid());
    queue.active.as_mut().unwrap().jumping = false;
    assert!(!queue.snapshot().valid());
    let step = queue.active.as_mut().unwrap();
    step.jumping = true;
    step.elapsed = 0.0;
    assert!(!queue.snapshot().valid());
}

#[test]
fn npc_relocation_defers_the_jump_tail_for_one_character_update() {
    use crate::world::test_support::{app, command, entity, event, page};
    let mut app = app(
        vec![event(
            1,
            1,
            vec![page(vec![
                command(24, 0),
                command(1, 0),
                command(1, 0),
                command(25, 0),
                command(32, 8),
            ])],
        )],
        false,
    );
    let npc = entity(&mut app, 1);
    app.update();
    let route = app
        .world()
        .get::<crate::world::RouteStepper>(npc)
        .unwrap()
        .clone();
    app.world_mut().write_message(crate::world::RelocateEvent {
        event_id: 1,
        x: 8,
        y: 9,
    });
    app.world_mut()
        .run_system_cached(crate::world::relocation::apply_relocate)
        .unwrap();
    assert_eq!(
        app.world().get::<crate::world::RouteStepper>(npc).unwrap(),
        &route
    );
    assert!(app.world().get::<MoveQueue>(npc).unwrap().jumping());
    app.update();
    assert!(!app.world().get::<MoveQueue>(npc).unwrap().busy());
    assert_eq!(
        app.world()
            .get::<crate::world::RouteStepper>(npc)
            .unwrap()
            .stop_count(),
        0
    );
    assert!(!app.world().resource::<crate::state::Switches>().get(8));
    app.update();
    assert!(app.world().resource::<crate::state::Switches>().get(8));
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile(), (8, 9));
}
