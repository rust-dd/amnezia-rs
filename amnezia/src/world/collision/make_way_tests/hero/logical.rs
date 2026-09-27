use super::*;
use crate::timing::{TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Debug, PartialEq)]
struct Sample {
    hero: ((i32, i32), u32, Vec2, u32),
    npcs: Vec<(u32, (i32, i32), Vec2, u32)>,
    calls: i32,
}

#[derive(Resource, Default)]
struct Trace(Vec<Sample>);

fn observe(
    data: Res<MapData>,
    variables: Res<crate::state::Variables>,
    heroes: Query<(&Player, &MoveQueue, &RouteStepper)>,
    npcs: Query<(&EventSprite, &MoveQueue, &RouteStepper)>,
    mut trace: ResMut<Trace>,
) {
    let (hero, queue, route) = heroes.single().unwrap();
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
    trace.0.push(Sample {
        hero: (
            hero.tile(),
            hero.frame,
            queue.ground_position(hero, &data),
            route.stop_count(),
        ),
        npcs,
        calls: variables.get(1),
    });
}

fn run(fps: u32) -> Vec<Sample> {
    let mut mover = gated(page(vec![command(1, 0)]));
    mover.move_route.repeat = true;
    let mut other = mover.clone();
    other.trigger = 4;
    other.commands = vec![counter()];
    let mut app = app(vec![at(1, 4, 5, mover), at(2, 6, 5, other)], false);
    app.add_plugins((TimingPlugin, LogicalPlugin))
        .insert_resource(MapData::for_test(200, 30))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .init_resource::<Trace>()
        .add_systems(
            Update,
            observe
                .after(crate::player::CameraFollow)
                .before(crate::dialogue::MessageUpdate),
        );
    app.update();
    app.world_mut().resource_mut::<Trace>().0.clear();
    app.world_mut()
        .resource_mut::<crate::state::Switches>()
        .set(7, true);
    move_hero(&mut app, false, 1);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / f64::from(fps),
    )));
    for _ in 0..fps {
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }
    app.world_mut().remove_resource::<Trace>().unwrap().0
}

#[test]
fn hero_npc_make_way_chains_have_identical_logical_traces_at_all_render_rates() {
    let expected = run(60);
    assert_eq!(expected.len(), 60);
    assert_eq!(expected[0].hero.0, (6, 5));
    assert_eq!(expected[0].npcs[0].1, (5, 5));
    assert_eq!(expected[0].npcs[1].1, (7, 5));
    assert_eq!(expected.last().unwrap().calls, 68);
    for fps in [15, 30, 144] {
        assert_eq!(run(fps), expected, "{fps} FPS");
    }
}
