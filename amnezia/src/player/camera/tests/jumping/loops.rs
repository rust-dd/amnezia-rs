use super::*;

fn fixture(size: (i32, i32), tile: (i32, i32)) -> (App, Entity, Vec2) {
    let (mut app, hero, _) = super::super::walking::fixture(tile);
    let mut map = MapData::for_test(size.0, size.1);
    map.scroll_type = 3;
    app.insert_resource(map);
    app.world_mut().resource_mut::<CameraPan>().recenter(true);
    app.update();
    let origin = app.world().resource::<CameraPan>().position.unwrap();
    (app, hero, origin)
}

fn jump_in_place(app: &mut App, hero: Entity) {
    let mut route = app.world_mut().get_mut::<RouteStepper>(hero).unwrap();
    route.set_speed(6);
    route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 24, 25]));
    for _ in 0..4 {
        app.update();
    }
    assert!(!app.world().get::<MoveQueue>(hero).unwrap().busy());
}

#[test]
fn odd_sized_loop_maps_round_the_canonical_phase_not_the_unwrapped_camera() {
    for (size, tile, direction, delta) in [
        ((21, 30), (0, 15), 1, Vec2::X),
        ((40, 17), (20, 0), 2, Vec2::NEG_Y),
    ] {
        let (mut app, hero, origin) = fixture(size, tile);
        app.world_mut()
            .resource_mut::<CameraPan>()
            .command(&[2, direction, 1, 1, 0]);
        for _ in 0..29 {
            app.update();
        }
        jump_in_place(&mut app, hero);
        let pan = app.world().resource::<CameraPan>();
        assert_eq!(pan.position, Some(origin + delta * 0.25));
        assert_eq!(pan.effects_position(), Some(origin + delta * 8.25));
    }
}

#[test]
fn negative_scroll_remainders_stay_signed_until_landing_normalizes_them() {
    let (mut app, hero, origin) = fixture((21, 30), (10, 15));
    app.world_mut()
        .resource_mut::<CameraPan>()
        .command(&[2, 3, 2, 4, 0]);
    for _ in 0..9 {
        app.update();
    }
    jump_in_place(&mut app, hero);
    let pan = app.world().resource::<CameraPan>();
    assert_eq!(pan.position, Some(origin - Vec2::X * 18.0));
    assert_eq!(pan.effects_position(), Some(origin - Vec2::X * 26.0));
}
