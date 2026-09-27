use super::*;

fn command(code: u32, params: Vec<i32>) -> amnezia_data::EventCommand {
    amnezia_data::EventCommand {
        code,
        params,
        indent: 0,
        string: String::new(),
    }
}

#[test]
fn foreground_waits_for_boarding_unboarding_ascent_and_descent_while_parallel_events_run() {
    for disembark in [false, true] {
        for index in 0..3 {
            let (mut app, hero) = app();
            test_support::direction(&mut app, DIR_RIGHT);
            app.world_mut()
                .get_mut::<RouteStepper>(hero)
                .unwrap()
                .set_speed(2);
            let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
            vehicles.set_location(index, 0, if disembark || index == 2 { 5 } else { 6 }, 5);
            vehicles.save.riding = disembark.then_some(index);
            vehicles.save.preboard_speed = 2;
            app.insert_resource(crate::interpreter::CommonEvents(vec![
                amnezia_data::CommonEvent {
                    id: 1,
                    name: String::new(),
                    trigger: 4,
                    switch_flag: false,
                    switch_id: 0,
                    commands: vec![command(10220, vec![0, 2, 2, 1, 0, 1])],
                },
            ]));
            app.world_mut()
                .resource_mut::<crate::interpreter::RunningEvent>()
                .start(
                    0,
                    vec![
                        command(10840, vec![]),
                        command(10220, vec![0, 1, 1, 1, 0, 1]),
                    ],
                );
            app.update();
            for frame in 1..=32 {
                assert_eq!(
                    app.world().resource::<Variables>().get(1),
                    0,
                    "{disembark}/{index}/{frame}"
                );
                assert_eq!(app.world().resource::<Variables>().get(2), frame);
                app.update();
            }
            assert_eq!(app.world().resource::<Variables>().get(1), 1);
            assert!(
                !app.world()
                    .resource::<crate::interpreter::RunningEvent>()
                    .active()
            );
        }
    }
}
