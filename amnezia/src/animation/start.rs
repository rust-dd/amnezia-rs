use super::*;
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct AnimationStart;

#[derive(Resource)]
struct Installed;

pub(super) fn register(app: &mut App) {
    app.insert_resource(Installed)
        .init_schedule(AnimationStart)
        .add_systems(
            AnimationStart,
            (
                resolve_map_animation,
                start_animations,
                track_active_animations,
            )
                .chain(),
        )
        .add_systems(
            Update,
            flush
                .in_set(AnimationSet::Start)
                .after(crate::interpreter::InterpreterStep)
                .after(AnimationSet::Advance),
        );
}

pub(crate) fn flush(world: &mut World) {
    if world.contains_resource::<Installed>() {
        world.run_schedule(AnimationStart);
    }
}
