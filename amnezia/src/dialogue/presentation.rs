use super::view;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct WindowPresentation;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Stage {
    Message,
    Inn,
}

#[derive(Resource)]
struct Installed;

struct PresentationPlugin;

impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Installed)
            .init_schedule(WindowPresentation)
            .configure_sets(WindowPresentation, (Stage::Message, Stage::Inn).chain())
            .add_systems(
                Update,
                flush
                    .in_set(super::DialogueView)
                    .after(crate::interpreter::InterpreterStep),
            );
    }
}

pub(crate) fn register<M>(
    app: &mut App,
    stage: Stage,
    systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
) {
    if !app.is_plugin_added::<PresentationPlugin>() {
        app.add_plugins(PresentationPlugin);
    }
    app.add_systems(WindowPresentation, systems.in_set(stage));
}

pub(super) fn register_windows(app: &mut App) {
    register(
        app,
        Stage::Message,
        (
            view::render_box,
            view::target_camera,
            view::render_reveal,
            view::prompts::render_cursor,
            super::position::latch,
            view::motion::render,
        )
            .chain(),
    );
}

pub(crate) fn flush(world: &mut World) {
    if world.contains_resource::<Installed>() {
        world.run_schedule(WindowPresentation);
    }
}

#[cfg(test)]
mod tests;
