use super::*;
use crate::world::RouteAction;

#[test]
fn a_floor_touch_runs_in_the_update_that_completes_the_step() {
    for forced in [false, true] {
        let mut app = app();
        let mut event = map_event(1, 1, vec![switch_cmd(1, 0, 0)]);
        event.x = 6;
        event.y = 5;
        app.insert_resource(MapEvents {
            events: vec![event],
        });
        let world = app.world_mut();
        if forced {
            *world
                .query::<&mut RouteStepper>()
                .single_mut(world)
                .unwrap() = RouteStepper::from_move_event(&[10001, 8, 0, 0, 1]).with_speed(6);
        } else {
            let mut queue = world.query::<&mut MoveQueue>().single_mut(world).unwrap();
            queue.set_step_secs(2.0 / 60.0);
            queue.push_step(RouteAction::Step {
                dx: 1,
                dy: 0,
                face: crate::tiles::DIR_RIGHT,
            });
        }
        app.update();
        assert!(!switch_on(&app, 1));
        app.update();
        assert!(switch_on(&app, 1), "forced={forced}");
        app.world_mut().resource_mut::<Switches>().set(1, false);
        app.update();
        assert!(!switch_on(&app, 1));
    }
}

#[test]
fn foreground_movement_wait_resumes_on_the_last_tween_update() {
    for vehicle in [false, true] {
        let mut app = app();
        if vehicle {
            app.add_plugins(crate::vehicles::VehiclePlugin)
                .init_resource::<crate::audio::CurrentBgm>();
            let mut vehicles = app.world_mut().resource_mut::<crate::vehicles::Vehicles>();
            vehicles.set_location(0, 0, 5, 6);
            vehicles.save.vehicles[0].speed = 6;
        }
        let reference = if vehicle { 10002 } else { 10001 };
        app.world_mut().resource_mut::<RunningEvent>().start(
            0,
            vec![
                cmd(11330, 0, vec![reference, 8, 0, 0, 28, 28, 1]),
                cmd(11340, 0, vec![]),
                switch_cmd(1, 0, 0),
            ],
        );
        app.update();
        app.update();
        assert!(!switch_on(&app, 1));
        app.update();
        assert!(switch_on(&app, 1), "vehicle={vehicle}");
    }
}
