use super::*;
use test_support::{direction, ticks, toggle};

#[test]
fn boarding_finishes_at_the_old_speed_and_disembarking_restores_it() {
    for speed in 1..=6 {
        let (mut app, hero) = app();
        direction(&mut app, DIR_RIGHT);
        app.world_mut()
            .get_mut::<RouteStepper>(hero)
            .unwrap()
            .set_speed(speed);
        app.world_mut()
            .resource_mut::<Vehicles>()
            .set_location(0, 0, 6, 5);
        toggle(&mut app);
        let frames = 256_u32.div_ceil(1 << (1 + speed));
        for _ in 0..frames - 1 {
            assert!(app.world().resource::<Vehicles>().save.boarding);
            assert_eq!(
                app.world().get::<RouteStepper>(hero).unwrap().speed(),
                speed
            );
            app.update();
        }
        assert!(app.world().resource::<Vehicles>().save.boarding);
        app.update();
        assert!(app.world().resource::<Vehicles>().aboard());
        assert_eq!(app.world().get::<RouteStepper>(hero).unwrap().speed(), 4);
        assert_eq!(app.world().get::<Player>(hero).unwrap().dir, DIR_LEFT);
        toggle(&mut app);
        assert!(app.world().resource::<Vehicles>().save.unboarding);
        assert!(!app.world().resource::<Vehicles>().riding());
        assert_eq!(
            app.world().get::<RouteStepper>(hero).unwrap().speed(),
            speed
        );
        assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (7, 5));
        ticks(&mut app, frames as usize);
        assert!(!app.world().resource::<Vehicles>().save.unboarding);
        assert_eq!(
            app.world()
                .resource::<crate::conditions::FieldSteps>()
                .count,
            0
        );
    }
}

#[test]
fn ship_wins_over_boat_and_a_moving_ship_can_be_boarded() {
    let (mut app, hero) = app();
    direction(&mut app, DIR_RIGHT);
    let data = app.world_mut().remove_resource::<MapData>().unwrap();
    let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
    vehicles.set_location(0, 0, 6, 5);
    vehicles.set_location(1, 0, 5, 5);
    let vehicles = &mut *vehicles;
    vehicles.motion[1].queue.begin_from(
        &mut vehicles.save.vehicles[1],
        &data,
        (5, 5),
        RouteAction::Step {
            dx: 1,
            dy: 0,
            face: DIR_RIGHT,
        },
    );
    app.insert_resource(data);
    toggle(&mut app);
    assert_eq!(app.world().resource::<Vehicles>().save.riding, Some(1));
    assert!(app.world().get::<MoveQueue>(hero).unwrap().busy());
}

#[test]
fn scripted_airship_toggle_cannot_interrupt_ascent_and_restores_speed_only_after_landing() {
    let (mut app, hero) = app();
    app.world_mut()
        .get_mut::<RouteStepper>(hero)
        .unwrap()
        .set_speed(2);
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_location(2, 0, 5, 5);
    toggle(&mut app);
    assert_eq!(app.world().get::<RouteStepper>(hero).unwrap().speed(), 5);
    for tick in 0..32 {
        toggle(&mut app);
        assert_eq!(
            app.world()
                .resource::<Vehicles>()
                .airship_ascent_remaining(),
            256 - tick * 8
        );
        app.update();
    }
    toggle(&mut app);
    ticks(&mut app, 31);
    assert!(app.world().resource::<Vehicles>().riding());
    assert_eq!(app.world().get::<RouteStepper>(hero).unwrap().speed(), 5);
    app.update();
    assert!(!app.world().resource::<Vehicles>().riding());
    assert_eq!(app.world().get::<RouteStepper>(hero).unwrap().speed(), 2);
    assert_eq!(app.world().get::<Player>(hero).unwrap().dir, DIR_DOWN);
}
