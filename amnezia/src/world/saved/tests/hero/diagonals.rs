use super::*;

fn step(app: &mut App) {
    app.world_mut().resource_scope(|world, data: Mut<MapData>| {
        let (mut hero, mut queue, mut route) = world
            .query::<(&mut Player, &mut MoveQueue, &mut RouteStepper)>()
            .single_mut(world)
            .unwrap();
        drive_route(
            &mut *hero,
            &mut queue,
            &mut route,
            (0, 0),
            1.0 / 60.0,
            |_, _, _, _, _| true,
        );
        queue.advance(&mut *hero, &data, 1.0 / 60.0);
    });
}

#[test]
fn a_saved_diagonal_route_continues_forward_without_losing_an_axis() {
    let (mut app, path, entity) = hero_app("hero-diagonal");
    app.world_mut()
        .get_mut::<RouteStepper>(entity)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 5, 11]));
    step(&mut app);
    assert_eq!(app.world().get::<Player>(entity).unwrap().tile(), (11, 11));
    save_and_load(&mut app);
    std::fs::remove_file(path).unwrap();
    for _ in 0..60 {
        step(&mut app);
    }
    assert_eq!(app.world().get::<Player>(entity).unwrap().tile(), (12, 12));
    assert!(!app.world().get::<MoveQueue>(entity).unwrap().busy());
    assert!(!app.world().get::<RouteStepper>(entity).unwrap().pending());
}
