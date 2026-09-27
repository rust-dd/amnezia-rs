use super::*;
use crate::timing::{TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Debug, PartialEq)]
struct Sample {
    hero: crate::world::saved::hero::HeroState,
    vehicles: VehicleSave,
    motion: saved::State,
    hero_tile: (i32, i32),
    camera: crate::player::saved_camera::CameraState,
}

#[derive(Resource, Default)]
struct Trace(Vec<Sample>);

fn observe(world: &mut World) {
    let hero = crate::world::saved::hero::snapshot(world).unwrap();
    let hero_tile = world.query::<&Player>().single(world).unwrap().tile();
    let vehicles = world.resource::<Vehicles>();
    let sample = Sample {
        hero,
        hero_tile,
        vehicles: vehicles.save.clone(),
        motion: vehicles.motion_snapshot(),
        camera: world.resource::<crate::player::CameraPan>().snapshot(),
    };
    world.resource_mut::<Trace>().0.push(sample);
}

fn run(fps: u32) -> Vec<Sample> {
    let (mut app, hero) = app();
    app.insert_resource(water_map(200, 100));
    app.world_mut()
        .get_mut::<Player>(hero)
        .unwrap()
        .set_tile(30, 30);
    app.world_mut().spawn((
        crate::world::MainCamera,
        Transform::default(),
        Projection::Orthographic(OrthographicProjection {
            area: Rect::new(-160.0, -120.0, 160.0, 120.0),
            ..OrthographicProjection::default_2d()
        }),
    ));
    app.add_plugins((TimingPlugin, LogicalPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .init_resource::<Trace>()
        .add_systems(
            Update,
            observe
                .after(VehicleStep)
                .before(crate::dialogue::MessageUpdate),
        );
    app.update();
    app.world_mut().resource_mut::<Trace>().0.clear();
    test_support::direction(&mut app, DIR_RIGHT);
    app.world_mut()
        .get_mut::<RouteStepper>(hero)
        .unwrap()
        .set_speed(2);
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_location(0, 0, 31, 30);
    test_support::toggle(&mut app);
    app.world_mut()
        .get_mut::<RouteStepper>(hero)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[
            10001, 8, 0, 0, 1, 24, 1, 1, 25, 32, 9,
        ]));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / f64::from(fps),
    )));
    for _ in 0..fps {
        app.update();
    }
    app.world_mut().remove_resource::<Trace>().unwrap().0
}

#[test]
fn boarding_rider_routes_vehicle_sync_and_scroll_match_at_every_render_rate() {
    let expected = run(60);
    assert_eq!(expected.len(), 60);
    assert!(expected[0].vehicles.boarding);
    assert!(!expected[31].vehicles.boarding);
    assert_eq!(expected[31].hero.route.speed(), 4);
    assert_eq!(expected.last().unwrap().hero_tile, (34, 30));
    for fps in [15, 30, 144] {
        assert_eq!(run(fps), expected, "{fps} FPS");
    }
}
