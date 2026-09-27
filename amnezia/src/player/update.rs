use super::{CameraFollow, Player, PlayerStep};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;

#[derive(Resource)]
pub(crate) struct Enabled;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CharacterUpdate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct PlayerPostUpdate;

pub(super) fn register(app: &mut App) {
    app.insert_resource(Enabled)
        .init_schedule(CharacterUpdate)
        .init_schedule(PlayerPostUpdate)
        .add_systems(
            Update,
            (
                advance
                    .in_set(PlayerStep)
                    .after(crate::interpreter::ParallelStep)
                    .after(crate::menu::MenuInput)
                    .after(crate::world::saved::RestoreCharacters)
                    .after(crate::vehicles::saved::RestoreVehicles)
                    .after(crate::appearance::PlayerGraphics)
                    .before(crate::vehicles::VehicleStep)
                    .before(crate::dialogue::MessageUpdate),
                finish
                    .in_set(CameraFollow)
                    .after(PlayerStep)
                    .before(crate::vehicles::VehicleStep)
                    .before(crate::screenfx::ScreenShakeSet)
                    .before(crate::dialogue::MessageUpdate),
            ),
        );
}

pub(crate) fn standalone(enabled: Option<Res<Enabled>>) -> bool {
    enabled.is_none()
}

pub(crate) fn character<M, S>(app: &mut App, systems: impl Fn() -> S) -> &mut App
where
    S: IntoScheduleConfigs<ScheduleSystem, M>,
{
    app.init_schedule(CharacterUpdate)
        .add_systems(Update, systems().run_if(standalone))
        .add_systems(CharacterUpdate, systems())
}

pub(crate) fn post<M, S>(app: &mut App, systems: impl Fn() -> S) -> &mut App
where
    S: IntoScheduleConfigs<ScheduleSystem, M>,
{
    app.init_schedule(PlayerPostUpdate)
        .add_systems(Update, systems().run_if(standalone))
        .add_systems(PlayerPostUpdate, systems())
}

pub(crate) fn early(world: &mut World) {
    if world.contains_resource::<Enabled>() {
        advance(world);
        finish(world);
    }
}

fn advance(world: &mut World) {
    if asynchronous_pause(world) {
        return;
    }
    let Some(entity) = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .ok()
    else {
        return;
    };
    if crate::world::update::claim(world, entity) {
        world.run_schedule(CharacterUpdate);
    }
}

fn finish(world: &mut World) {
    if asynchronous_pause(world) {
        return;
    }
    crate::picture::apply_pending(world);
    // Pan and Cancel capture also run on repeated MakeWay visits.
    world.run_schedule(PlayerPostUpdate);
}

fn asynchronous_pause(world: &World) -> bool {
    world
        .get_resource::<crate::interpreter::continuation::Continuation>()
        .is_some_and(|state| state.characters_paused(false))
}
