use super::*;
use crate::world::RouteStepper;

mod logical;

pub(super) fn fixture(tile: (i32, i32)) -> (App, Entity, Vec2) {
    let (app, hero) = crate::world::test_support::camera_app(tile);
    let position = app.world().resource::<CameraPan>().position.unwrap();
    (app, hero, position)
}

fn force(app: &mut App, hero: Entity, speed: u32, direction: i32) {
    let mut route = app.world_mut().get_mut::<RouteStepper>(hero).unwrap();
    route.set_speed(speed);
    route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, direction]));
}

fn position(app: &App) -> Vec2 {
    app.world().resource::<CameraPan>().position.unwrap()
}

#[test]
fn a_live_heading_change_does_not_scroll_by_the_sprite_offset_jump() {
    let (mut app, hero, origin) = fixture((20, 15));
    force(&mut app, hero, 4, 1);
    app.update();
    assert_eq!(position(&app), origin + Vec2::X * 2.0);
    let world = app.world_mut();
    let (mut hero, mut route) = world
        .query::<(&mut Player, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    route.set_direction(&mut *hero, crate::tiles::DIR_LEFT);
    app.update();
    assert_eq!(position(&app), origin + Vec2::X * 2.0);
}

#[test]
fn walking_uses_the_logical_destination_and_does_not_cap_scroll_to_its_gap() {
    let (mut app, hero, origin) = fixture((20, 15));
    app.world_mut().resource_mut::<CameraPan>().position = Some(origin + Vec2::X * 15.75);
    force(&mut app, hero, 4, 1);
    app.update();
    assert_eq!(position(&app), origin + Vec2::X * 17.75);
    app.update();
    assert_eq!(position(&app), origin + Vec2::X * 17.75);
}

#[test]
fn scroll_direction_uses_half_map_modulo_even_on_a_nonlooping_map() {
    let (mut app, hero, origin) = fixture((5, 15));
    app.world_mut().resource_mut::<CameraPan>().position = Some(Vec2::new(160.0, origin.y));
    force(&mut app, hero, 4, 3);
    app.update();
    assert_eq!(position(&app), Vec2::new(160.0, origin.y));
}

#[test]
fn a_faster_last_tick_scrolls_its_full_amount_not_only_the_remaining_distance() {
    let (mut app, hero, origin) = fixture((20, 15));
    force(&mut app, hero, 1, 1);
    for _ in 0..63 {
        app.update();
    }
    assert_eq!(position(&app), origin + Vec2::X * 15.75);
    assert!(app.world().get::<MoveQueue>(hero).unwrap().busy());
    app.world_mut()
        .get_mut::<RouteStepper>(hero)
        .unwrap()
        .set_speed(6);
    app.update();
    assert!(!app.world().get::<MoveQueue>(hero).unwrap().busy());
    assert_eq!(position(&app), origin + Vec2::X * 23.75);
}

#[test]
fn all_walking_directions_and_speeds_scroll_the_original_subpixel_amount() {
    let directions = [
        (0, -1),
        (1, 0),
        (0, 1),
        (-1, 0),
        (1, -1),
        (1, 1),
        (-1, 1),
        (-1, -1),
    ];
    for (direction, (dx, dy)) in directions.into_iter().enumerate() {
        for speed in 1..=6 {
            let (mut app, hero, origin) = fixture((20, 15));
            force(&mut app, hero, speed, direction as i32);
            let frames = 256 / (1 << (1 + speed));
            for frame in 1..=frames {
                app.update();
                assert_eq!(
                    position(&app),
                    origin
                        + Vec2::new(dx as f32, -dy as f32) * (16.0 * frame as f32 / frames as f32),
                    "direction {direction}, speed {speed}, frame {frame}"
                );
            }
            assert!(!app.world().get::<MoveQueue>(hero).unwrap().busy());
        }
    }
}

#[test]
fn locking_suppresses_walk_scroll_but_not_pan_and_unlock_does_not_recenter() {
    let (mut app, hero, origin) = fixture((20, 15));
    let mut pan = app.world_mut().resource_mut::<CameraPan>();
    pan.locked = true;
    pan.target = Vec2::Y * 16.0;
    force(&mut app, hero, 4, 1);
    for frame in 1..=5 {
        app.update();
        assert_eq!(position(&app), origin + Vec2::Y * frame as f32);
    }
    app.world_mut().resource_mut::<CameraPan>().locked = false;
    app.update();
    assert_eq!(position(&app), origin + Vec2::new(2.0, 6.0));
}

#[test]
fn walking_across_all_loop_seams_keeps_a_continuous_camera_position() {
    for (tile, direction, delta) in [
        ((39, 15), 1, Vec2::X),
        ((0, 15), 3, Vec2::NEG_X),
        ((20, 0), 0, Vec2::Y),
        ((20, 29), 2, Vec2::NEG_Y),
    ] {
        let (mut app, hero, _) = fixture(tile);
        app.world_mut().resource_mut::<MapData>().scroll_type = 3;
        app.world_mut().resource_mut::<CameraPan>().recenter(false);
        app.update();
        let origin = position(&app);
        force(&mut app, hero, 4, direction);
        for frame in 1..=8 {
            app.update();
            assert_eq!(position(&app), origin + delta * (2 * frame) as f32);
        }
    }
}

#[test]
fn a_walk_initializes_a_missing_camera_at_its_origin_before_scrolling() {
    let (mut app, hero, origin) = fixture((20, 15));
    app.world_mut().resource_mut::<CameraPan>().recenter(false);
    force(&mut app, hero, 4, 1);
    app.update();
    assert_eq!(position(&app), origin + Vec2::X * 2.0);
}

#[test]
fn serialized_motion_and_camera_resume_live_speed_changes_without_a_scroll_jump() {
    use crate::player::saved_camera::CameraState;
    use crate::world::saved::hero::{HeroState, snapshot};
    let (mut original, original_hero, origin) = fixture((20, 15));
    force(&mut original, original_hero, 1, 1);
    for _ in 0..31 {
        original.update();
    }
    let saved = ron::to_string(&(
        original.world().resource::<CameraPan>().snapshot(),
        snapshot(original.world_mut()).unwrap(),
    ))
    .unwrap();
    let (camera, state) = ron::from_str::<(CameraState, HeroState)>(&saved).unwrap();
    assert!(camera.valid() && state.valid());
    let (mut restored, restored_hero, _) = fixture((21, 15));
    restored.insert_resource(camera.into_pan());
    restored
        .world_mut()
        .entity_mut(restored_hero)
        .insert((state.motion.into_queue(), state.route));
    {
        let mut hero = restored
            .world_mut()
            .get_mut::<Player>(restored_hero)
            .unwrap();
        hero.frame = state.frame;
        hero.dir = crate::tiles::DIR_RIGHT;
    }
    for (app, hero) in [
        (&mut original, original_hero),
        (&mut restored, restored_hero),
    ] {
        app.world_mut()
            .get_mut::<RouteStepper>(hero)
            .unwrap()
            .set_speed(6);
    }
    for _ in 0..10 {
        original.update();
        restored.update();
        assert_eq!(
            original.world().resource::<CameraPan>().snapshot(),
            restored.world().resource::<CameraPan>().snapshot()
        );
        assert_eq!(
            snapshot(original.world_mut()),
            snapshot(restored.world_mut())
        );
    }
    assert_eq!(position(&original), origin + Vec2::X * 23.75);
}
