use super::*;
use crate::world::{EventSprite, MapData, MoveQueue, drive_route};

mod directions;

fn character() -> EventSprite {
    EventSprite {
        id: 5,
        tile_x: 10,
        tile_y: 9,
        dir: DIR_DOWN,
        frame: 1,
        charset: "Chara4".into(),
        index: 0,
        layer: 0,
    }
}

#[test]
fn original_airship_jump_crosses_the_gap_and_reaches_the_graphic_change() {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0094.ron",
        crate::assets::asset_root()
    ));
    let command = map
        .events
        .iter()
        .flat_map(|event| &event.pages)
        .flat_map(|page| &page.commands)
        .find(|c| c.code == 11330 && c.params.first() == Some(&5) && c.params.contains(&24))
        .unwrap();
    let mut stepper = RouteStepper::from_move_event(&command.params).with_speed(3);
    let mut character = character();
    let mut queue = MoveQueue::default();
    let data = MapData::for_test(20, 16);
    let landings = std::cell::Cell::new(0);
    for _ in 0..120 {
        let position = character.tile();
        drive_route(
            &mut character,
            &mut queue,
            &mut stepper,
            (9, 4),
            1.0 / FPS,
            |_, dx, dy, jumping, _| {
                if jumping {
                    landings.set(landings.get() + 1);
                    assert_eq!((position.0 + dx, position.1 + dy), (15, 15));
                }
                true
            },
        );
        queue.advance(&mut character, &data, 1.0 / FPS);
    }
    assert_eq!(landings.get(), 1);
    assert_eq!(character.tile(), (15, 15));
    assert_eq!((&*character.charset, character.index), ("Torch", 1));
    assert_eq!(character.dir, DIR_DOWN);
    assert!(!stepper.active() && !queue.busy());
    assert_eq!(
        stepper.speed(),
        4,
        "speed changes inside the jump block are ignored"
    );
}

#[test]
fn blocked_jumps_retry_as_a_whole_or_skip_to_after_the_landing() {
    for skippable in [0, 1] {
        let mut stepper = RouteStepper::from_move_event(&[0, 8, 0, skippable, 24, 1, 1, 25, 32, 8]);
        let mut character = character();
        let mut effects = Vec::new();
        assert!(
            stepper
                .advance(
                    &mut character,
                    (0, 0),
                    &|_, _, _, jumping, _| {
                        assert!(jumping);
                        false
                    },
                    &mut effects
                )
                .is_none()
        );
        if skippable == 0 {
            assert!(effects.is_empty());
            assert!(matches!(
                stepper.advance(&mut character, (0, 0), &|_, _, _, _, _| true, &mut effects),
                Some((RouteAction::Jump { dx: 2, dy: 0, .. }, _))
            ));
            stepper.advance(&mut character, (0, 0), &|_, _, _, _, _| true, &mut effects);
        }
        assert!(matches!(effects.as_slice(), [StepEffect::Switch(8, true)]));
    }
}

#[test]
fn facing_lock_preserves_the_sprite_but_forward_uses_the_movement_direction() {
    let mut stepper = RouteStepper::from_move_event(&[0, 8, 0, 0, 26, 1, 11, 27, 0]);
    let mut character = character();
    for (dx, dy, face) in [(1, 0, DIR_DOWN), (1, 0, DIR_DOWN), (0, -1, DIR_UP)] {
        let action = stepper.advance(
            &mut character,
            (0, 0),
            &|_, _, _, _, _| true,
            &mut Vec::new(),
        );
        assert!(
            matches!(action, Some((RouteAction::Step { dx: x, dy: y, face: f }, _)) if (x, y, f) == (dx, dy, face))
        );
    }
}

#[test]
fn jump_tween_lifts_the_character_without_walking_frames() {
    let data = MapData::for_test(20, 16);
    let mut character = character();
    let mut queue = MoveQueue::default();
    queue.set_step_secs(1.0);
    queue.enqueue_route([RouteAction::Jump {
        dx: 4,
        dy: 2,
        face: DIR_DOWN,
    }]);
    let start = queue.advance(&mut character, &data, 0.0).unwrap();
    let midpoint = queue.advance(&mut character, &data, 0.5).unwrap();
    let end = queue.advance(&mut character, &data, 0.5).unwrap();
    assert_eq!(midpoint.x, (start.x + end.x) / 2.0);
    assert_eq!(midpoint.y, (start.y + end.y) / 2.0 + 16.0);
    assert_eq!(character.frame, 1);
    assert_eq!(character.tile(), (14, 11));
}
