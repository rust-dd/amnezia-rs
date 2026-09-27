mod advance;
use advance::advance;
pub(crate) use advance::{begin_update, early};
mod boarding;
pub(crate) use boarding::flush;
use boarding::keyboard;
mod flight;
#[cfg(test)]
mod landing_tests;
#[cfg(test)]
mod make_way_tests;
mod model;
mod obstacles;
#[cfg(test)]
mod relocation_tests;
mod render;
pub(crate) mod rider;
#[cfg(test)]
mod rider_tests;
pub(crate) mod saved;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;

pub use model::{VehicleSave, Vehicles};
pub(crate) use render::VehicleSprite;

use crate::assets::{asset_root, load_ron};
use crate::audio::{AudioRequest, CurrentBgm};
use crate::player::Player;
use crate::state::{Inventory, Party, Switches, Variables, active_page};
#[cfg(test)]
use crate::tiles::DIR_RIGHT;
use crate::tiles::{DIR_DOWN, DIR_LEFT};
#[cfg(test)]
use crate::world::drive_route;
use crate::world::{
    Character, MapData, MapEvents, MoveGuards, MoveQueue, RouteAction, RouteStepper, StepEffect,
    dir_delta, step_secs_for_speed,
};
use amnezia_data::{MusicDef, SystemDef};
use bevy::prelude::*;

#[derive(Resource)]
struct VehicleMusic([MusicDef; 3]);

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VehicleInput;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VehicleDisplay;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VehicleSync;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VehicleStep;

pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        let system = load_ron::<SystemDef>(&format!("{}/system.ron", asset_root()));
        crate::player::update::character(app, || {
            (
                rider::flight.before(crate::world::update::HeroRouteStep),
                keyboard
                    .in_set(VehicleInput)
                    .after(crate::world::update::HeroRouteStep)
                    .after(crate::player::PlayerInput)
                    .before(crate::player::PlayerStep),
            )
        });
        crate::player::update::post(app, || {
            rider::sync
                .in_set(VehicleSync)
                .before(crate::player::CameraFollow)
        });
        app.init_resource::<Vehicles>()
            .init_resource::<advance::Updates>()
            .insert_resource(VehicleMusic([
                system.boat_music,
                system.ship_music,
                system.airship_music,
            ]))
            .add_systems(
                Update,
                advance
                    .in_set(VehicleStep)
                    .after(saved::RestoreVehicles)
                    .after(crate::player::PlayerStep)
                    .after(crate::player::CameraFollow)
                    .after(VehicleInput),
            )
            .add_systems(
                PostUpdate,
                render::draw
                    .in_set(VehicleDisplay)
                    .after(crate::player::CameraFollow)
                    .after(crate::screenfx::ScreenShakeSet)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
    }
}
