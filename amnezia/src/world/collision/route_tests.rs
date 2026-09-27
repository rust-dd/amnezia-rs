use super::*;
use crate::player::Player;
use crate::world::{Character, MoveQueue, drive_route};
use amnezia_data::{MoveCommandDef, MoveRouteDef};

#[test]
fn instant_route_graphic_and_through_changes_reach_the_collision_gate() {
    let mut data = MapData::for_test(5, 5);
    data.passages_up[3] = 0;
    let events = MapEvents::default();
    let state = (
        &Switches::default(),
        &Variables::default(),
        &Party::default(),
        &Inventory::default(),
    );
    let bodies = CollisionBodies::default();
    let collision = MapCollision::new(&data, &events, state, &bodies);
    for through in [false, true] {
        let commands = vec![
            MoveCommandDef {
                code: 34,
                params: vec![3],
                string: String::new(),
            },
            MoveCommandDef {
                code: if through { 36 } else { 37 },
                params: vec![],
                string: String::new(),
            },
            MoveCommandDef {
                code: 1,
                params: vec![],
                string: String::new(),
            },
        ];
        let mut route = RouteStepper::from_page(
            &MoveRouteDef {
                commands,
                repeat: false,
                skippable: false,
            },
            4,
            8,
        );
        let mut character = EventSprite {
            id: 1,
            tile_x: 2,
            tile_y: 2,
            dir: 2,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
            layer: 0,
        };
        let result = drive_route(
            &mut character,
            &mut MoveQueue::default(),
            &mut route,
            (3, 2),
            |ch, dx, dy, jumping, through| {
                assert!(ch.charset().is_empty());
                assert_eq!(ch.index(), 3);
                collision.can_move(
                    ch.tile(),
                    (ch.tile_x + dx, ch.tile_y + dy),
                    Mover::event(ch, through),
                    Some((3, 2)),
                    jumping,
                )
            },
        );
        assert_eq!(result.moved, through.then_some((1, 0)));
    }
}

#[test]
fn through_forced_steps_and_jumps_retry_at_nonlooping_boundaries() {
    let data = MapData::for_test(5, 5);
    let events = MapEvents::default();
    let state = (
        &Switches::default(),
        &Variables::default(),
        &Party::default(),
        &Inventory::default(),
    );
    let bodies = CollisionBodies::default();
    let collision = MapCollision::new(&data, &events, state, &bodies);
    for codes in [vec![36, 3], vec![36, 24, 3, 25]] {
        let mut params = vec![10001, 8, 0, 0];
        params.extend(codes);
        let mut route = RouteStepper::from_move_event(&params);
        let mut player = Player {
            tile_x: 0,
            tile_y: 2,
            dir: 2,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
        };
        let mut queue = MoveQueue::default();
        let result = drive_route(
            &mut player,
            &mut queue,
            &mut route,
            (0, 0),
            |ch, dx, dy, jumping, through| {
                collision.can_move(ch.tile(), (dx, 2 + dy), Mover::hero(through), None, jumping)
            },
        );
        assert!(result.moved.is_none());
        assert!(!queue.busy());
        assert!(route.pending());
    }
}
