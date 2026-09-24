use super::GameFrames;
use bevy::app::MainScheduleOrder;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;
use std::time::Duration;

mod engine;
mod input;
pub(crate) use engine::EnginePlugin;
#[cfg(test)]
mod tests;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct GamePreUpdate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct GamePostUpdate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct LogicalUpdate;

#[derive(Resource, Default)]
pub(super) struct Step {
    pub(super) advancing: bool,
}

#[derive(Resource, Default)]
struct Clock {
    initialized: bool,
    time: Time,
    input: input::Buffered,
}

pub(crate) struct LogicalPlugin;

impl Plugin for LogicalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Step>()
            .init_resource::<Clock>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(Time::<Fixed>::from_hz(60.0))
            .init_schedule(GamePreUpdate)
            .init_schedule(GamePostUpdate)
            .init_schedule(Update)
            .add_systems(LogicalUpdate, update);
        let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
        let update = order
            .labels
            .iter_mut()
            .find(|label| **label == Update.intern())
            .expect("the main schedule must contain Update");
        *update = LogicalUpdate.intern();
    }
}

fn standalone(step: Option<Res<Step>>) -> bool {
    step.is_none()
}

pub(crate) fn pre<M, S>(app: &mut App, systems: impl Fn() -> S) -> &mut App
where
    S: IntoScheduleConfigs<ScheduleSystem, M>,
{
    app.add_systems(PreUpdate, systems().run_if(standalone))
        .add_systems(GamePreUpdate, systems())
}

pub(crate) fn post<M, S>(app: &mut App, systems: impl Fn() -> S) -> &mut App
where
    S: IntoScheduleConfigs<ScheduleSystem, M>,
{
    app.add_systems(PostUpdate, systems().run_if(standalone))
        .add_systems(GamePostUpdate, systems())
}

fn update(world: &mut World) {
    engine::update(world);
    let render_time = *world.resource::<Time>();
    let input = world.resource::<ButtonInput<KeyCode>>().clone();
    world.resource_scope(|world, mut clock: Mut<Clock>| {
        clock.input.capture(&input);
        let mut frames = world.resource_mut::<GameFrames>();
        let mut budget = *frames;
        budget.advance(render_time.delta_secs_f64());
        let ticks = budget.frame.wrapping_sub(frames.frame);
        frames.fraction = budget.fraction;
        let bootstrap = !clock.initialized && ticks == 0;
        clock.initialized = true;
        for _ in 0..ticks.max(u32::from(bootstrap)) {
            world.resource_mut::<Step>().advancing = !bootstrap;
            clock.time.advance_by(if bootstrap {
                Duration::ZERO
            } else {
                Duration::from_secs_f64(1.0 / 60.0)
            });
            world.insert_resource(clock.time);
            world.insert_resource(clock.input.take());
            world.run_schedule(GamePreUpdate);
            world.run_schedule(Update);
            world.run_schedule(GamePostUpdate);
        }
    });
    world.insert_resource(render_time);
}
