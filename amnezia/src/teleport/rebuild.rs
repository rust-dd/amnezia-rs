use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct MapRestoration;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Stage {
    Reset,
    Characters,
    State,
    Animation,
    Music,
    SavedMusic,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Restored;

#[derive(Resource)]
struct Installed;

struct RebuildPlugin;

impl Plugin for RebuildPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Installed)
            .init_schedule(MapRestoration)
            .configure_sets(
                MapRestoration,
                (
                    Stage::Reset,
                    Stage::Characters,
                    Stage::State,
                    Stage::Animation,
                    Stage::Music,
                    Stage::SavedMusic,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                flush
                    .in_set(Restored)
                    .in_set(crate::world::saved::RestoreCharacters)
                    .in_set(crate::vehicles::saved::RestoreVehicles)
                    .in_set(crate::screenfx::saved::RestoreScreen)
                    .in_set(crate::screenfx::MapScreenReset)
                    .in_set(crate::map_bgm::MapMusic)
                    .after(super::MapTransfer)
                    .before(crate::interpreter::ParallelStep)
                    .before(crate::player::CameraFollow)
                    .before(crate::screenfx::ScreenAdvance)
                    .before(crate::animation::AnimationSet::Advance),
            );
    }
}

pub(crate) fn register<M>(
    app: &mut App,
    stage: Stage,
    systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
) {
    if !app.is_plugin_added::<RebuildPlugin>() {
        app.add_plugins(RebuildPlugin);
    }
    app.add_systems(MapRestoration, systems.in_set(stage));
}

pub(crate) fn flush(world: &mut World) {
    if world.contains_resource::<Installed>() {
        world.run_schedule(MapRestoration);
    }
}

#[cfg(test)]
mod tests;
