use super::*;
use crate::conditions::{ConditionsPlugin, FieldSteps};
use crate::timing::{TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

fn run(fps: u32) -> (i32, u32, Vec3, u64) {
    let mut app = movement_app(vec![]);
    app.add_plugins((TimingPlugin, LogicalPlugin, ConditionsPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .insert_resource(MapData::for_test(200, 30))
        .insert_resource(crate::gamedata::GameData {
            actors: vec![],
            items: vec![],
            skills: vec![],
        })
        .init_resource::<crate::progression::Progression>()
        .init_resource::<crate::vehicles::Vehicles>()
        .init_resource::<crate::vitals::Vitals>();
    {
        let world = app.world_mut();
        let mut route = world
            .query::<&mut RouteStepper>()
            .single_mut(world)
            .unwrap();
        *route = RouteStepper::default().with_speed(6);
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / f64::from(fps),
    )));
    for _ in 0..fps {
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }
    let world = app.world_mut();
    let (hero, transform) = world
        .query::<(&Player, &Transform)>()
        .single(world)
        .unwrap();
    (
        hero.tile_x,
        hero.frame,
        transform.translation,
        world.resource::<FieldSteps>().count,
    )
}

#[test]
fn fast_keyboard_movement_and_field_steps_match_at_every_render_rate() {
    let expected = run(60);
    assert!(expected.3 > 15, "multiple tile starts per 15 FPS render");
    for fps in [15, 30, 144] {
        assert_eq!(run(fps), expected, "{fps} FPS");
    }
}
