use super::*;

fn hero(tile: (i32, i32)) -> crate::player::Player {
    crate::player::Player {
        tile_x: tile.0,
        tile_y: tile.1,
        dir: DIR_RIGHT,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    }
}

#[test]
fn rider_sync_copies_only_remaining_step_and_keeps_a_vehicles_jump_origin_and_clock() {
    for legacy in [false, true] {
        let mut old = MapData::for_test(20, 15);
        old.scroll_type = 3;
        let data = MapData::for_test(40, 30);
        let mut vehicle = hero((19, 14));
        let mut queue = MoveQueue::default();
        queue.use_character_motion(1, DIR_RIGHT);
        queue.begin_from(
            &mut vehicle,
            &old,
            (19, 14),
            RouteAction::Jump {
                dx: 2,
                dy: 2,
                face: DIR_RIGHT,
            },
        );
        let active = queue.active.as_mut().unwrap();
        active.subpixels.as_mut().unwrap().fraction = 0.25;
        if legacy {
            active.jump_origin = None;
        }
        let mut rider = hero((7, 9));
        let mut motion = MoveQueue::default();
        motion.use_character_motion(4, DIR_RIGHT);
        motion.begin_from(
            &mut rider,
            &data,
            (7, 9),
            RouteAction::Step {
                dx: 1,
                dy: 0,
                face: DIR_RIGHT,
            },
        );
        motion.advance(&mut rider, &data, 1.0 / 60.0);
        let previous = vehicle.tile();
        vehicle.set_tile(8, 9);
        queue.sync_remaining(&motion, previous, &vehicle, &data);
        let active = queue.active.as_ref().unwrap();
        assert_eq!(active.jump_origin, Some((-1, -1)));
        assert_eq!(active.subpixels.unwrap().remaining, 224);
        assert_eq!(active.subpixels.unwrap().fraction, 0.25);
        assert_eq!(queue.kinematics.unwrap().speed, 1);
        assert!(queue.jumping() && !motion.jumping());
        assert!(queue.snapshot().valid());
        assert_eq!(
            queue.render_position(&vehicle, &data).y - queue.ground_position(&vehicle, &data).y,
            8.0
        );
        queue.sync_remaining(&MoveQueue::default(), vehicle.tile(), &vehicle, &data);
        assert!(queue.jumping() && queue.busy());
        assert!(queue.snapshot().valid());
        assert_eq!(queue.render_position(&vehicle, &data), center(&data, 8, 9));
        queue.advance(&mut vehicle, &data, 1.0 / 60.0);
        assert!(!queue.busy());
    }
}
