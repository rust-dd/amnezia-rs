use super::*;

pub(crate) fn rider_app(data: MapData, tile: (i32, i32)) -> (App, Entity) {
    let mut app = crate::world::test_support::app(vec![], false);
    app.insert_resource(data)
        .init_resource::<CurrentBgm>()
        .init_resource::<crate::system_bgm::SystemBgm>()
        .init_resource::<crate::conditions::FieldSteps>()
        .add_plugins(VehiclePlugin);
    let world = app.world_mut();
    for index in 0..3 {
        world
            .resource_mut::<Vehicles>()
            .set_location(index, 99, 0, 0);
    }
    let hero = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    world
        .get_mut::<Player>(hero)
        .unwrap()
        .set_tile(tile.0, tile.1);
    (app, hero)
}

pub(crate) fn toggle(app: &mut App) {
    app.world_mut().resource_mut::<Vehicles>().toggle_pending = true;
    flush(app.world_mut());
}

pub(crate) fn direction(app: &mut App, direction: u32) {
    let world = app.world_mut();
    let (mut hero, mut route) = world
        .query::<(&mut Player, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    route.set_direction(&mut *hero, direction);
}

pub(crate) fn ticks(app: &mut App, count: usize) {
    for _ in 0..count {
        app.update();
    }
}

pub(crate) fn passage_app(index: usize, manual: bool) -> App {
    if manual {
        let (mut app, _) = rider_app(MapData::for_test(10, 10), (4, 4));
        let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
        vehicles.set_location(index, 0, 4, 4);
        vehicles.save.riding = Some(index);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        return app;
    }
    let mut app = crate::interpreter::tests::interp_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ))
    .init_resource::<Vehicles>()
    .init_resource::<CurrentBgm>()
    .init_resource::<crate::system_bgm::SystemBgm>()
    .insert_resource(VehicleMusic(std::array::from_fn(|_| default())))
    .add_systems(
        Update,
        (keyboard, advance)
            .chain()
            .after(crate::interpreter::InterpreterStep),
    );
    app.update();
    {
        let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
        for other in 0..3 {
            vehicles.set_location(other, 99, 0, 0);
        }
        vehicles.set_location(index, 0, 4, 4);
        vehicles.save.riding = manual.then_some(index);
    }
    if manual {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
    } else {
        app.world_mut().resource_mut::<Vehicles>().set_route(
            10002 + index as i32,
            RouteStepper::from_move_event(&[10002 + index as i32, 8, 0, 0, 1]),
        );
    }
    app
}
