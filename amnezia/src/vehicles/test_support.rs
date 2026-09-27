use super::*;

pub(crate) fn passage_app(index: usize, manual: bool) -> App {
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
