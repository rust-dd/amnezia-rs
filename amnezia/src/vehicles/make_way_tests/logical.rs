use super::*;
use crate::timing::{TimingPlugin, logical::LogicalPlugin};
use crate::world::EventSprite;
use crate::world::test_support::gated;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Debug, PartialEq)]
struct Sample {
    vehicle: ((i32, i32), Vec2, u32, u32),
    npcs: Vec<(u32, (i32, i32), Vec2, u32)>,
    calls: i32,
}

#[derive(Resource, Default)]
struct Trace(Vec<Sample>);

fn observe(
    data: Res<MapData>,
    variables: Res<Variables>,
    vehicles: Res<Vehicles>,
    npcs: Query<(&EventSprite, &MoveQueue, &RouteStepper)>,
    mut trace: ResMut<Trace>,
) {
    let mut npcs = npcs
        .iter()
        .map(|(npc, queue, route)| {
            (
                npc.id,
                npc.tile(),
                queue.ground_position(npc, &data),
                route.stop_count(),
            )
        })
        .collect::<Vec<_>>();
    npcs.sort_by_key(|npc| npc.0);
    let vehicle = &vehicles.save.vehicles[0];
    let motion = &vehicles.motion[0];
    trace.0.push(Sample {
        vehicle: (
            vehicle.tile(),
            motion.queue.ground_position(vehicle, &data),
            vehicle.frame,
            motion.route.stop_count(),
        ),
        npcs,
        calls: variables.get(1),
    });
}

fn run(fps: u32) -> Vec<Sample> {
    let mut mover = gated(page(vec![command(1, 0)]));
    mover.move_route.repeat = true;
    let mut target = mover.clone();
    target.trigger = 4;
    target.commands = vec![counter()];
    let mut app = app(vec![event(1, 1, vec![mover]), event(2, 3, vec![target])]);
    app.add_plugins((TimingPlugin, LogicalPlugin))
        .insert_resource(water_map(200, 20))
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
    app.world_mut().resource_mut::<Switches>().set(7, true);
    vehicle(&mut app, 0, 2, 1, &[1]);
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_route(10002, RouteStepper::from_move_event(&[10002, 8, 1, 0, 1]));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / f64::from(fps),
    )));
    for _ in 0..fps {
        app.update();
    }
    app.world_mut().remove_resource::<Trace>().unwrap().0
}

#[test]
fn vehicle_make_way_has_identical_sixty_update_traces_at_every_render_rate() {
    let expected = run(60);
    assert_eq!(expected.len(), 60);
    assert_eq!(expected[0].vehicle.0, (3, 1));
    assert_eq!(expected[0].npcs[0].1, (2, 1));
    assert_eq!(expected[0].npcs[1].1, (4, 1));
    assert_eq!(expected.last().unwrap().calls, 68);
    for fps in [15, 30, 144] {
        assert_eq!(run(fps), expected, "{fps} FPS");
    }
}
