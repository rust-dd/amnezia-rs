use super::{Vehicles, model::Motion};
use crate::world::{MapData, MapRebuilt, RouteStepper, saved::MotionState};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

mod migration;
pub(crate) use migration::migrate_rider;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SavedMotion {
    queue: MotionState,
    route: RouteStepper,
    pixel: Option<[f32; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct State {
    motion: [SavedMotion; 3],
}

impl Vehicles {
    pub(crate) fn motion_snapshot(&self) -> State {
        State {
            motion: std::array::from_fn(|index| {
                let motion = &self.motion[index];
                SavedMotion {
                    queue: motion.queue.snapshot(),
                    route: motion.route.clone(),
                    pixel: motion.pixel.map(|pixel| pixel.to_array()),
                }
            }),
        }
    }
}

impl State {
    pub(crate) fn valid(&self, vehicles: &super::VehicleSave) -> bool {
        vehicles.valid()
            && self.motion.iter().all(|motion| {
                motion.queue.valid()
                    && motion.route.valid()
                    && motion.pixel.iter().flatten().all(|value| value.is_finite())
            })
    }
}

impl super::VehicleSave {
    pub(crate) fn valid(&self) -> bool {
        self.riding.is_none_or(|index| index < 3)
            && (!self.boarding || self.riding.is_some())
            && (1..=6).contains(&self.preboard_speed)
            && self.airship_flight.valid()
            && self.vehicles.iter().all(|vehicle| {
                vehicle.dir < 4
                    && vehicle.frame < 4
                    && (1..=6).contains(&vehicle.speed)
                    && vehicle.definition.index < 8
                    && vehicle.definition.x <= i32::MAX as u32
                    && vehicle.definition.y <= i32::MAX as u32
            })
    }
}

impl SavedMotion {
    fn into_motion(mut self) -> Motion {
        self.route.restore_stop_clock(None);
        Motion {
            queue: self.queue.into_queue(),
            alpha: self.route.alpha(),
            route: self.route,
            pixel: self.pixel.map(Vec2::from_array),
        }
    }
}

#[derive(Resource)]
pub(crate) struct Pending {
    map_id: u32,
    state: State,
}

pub(crate) fn prepare(world: &mut World, map_id: u32, state: Option<State>) {
    world.remove_resource::<Pending>();
    if let Some(state) = state {
        world.insert_resource(Pending { map_id, state });
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RestoreVehicles;

pub(crate) fn register(app: &mut App) {
    app.add_systems(
        Update,
        restore
            .in_set(RestoreVehicles)
            .after(crate::teleport::MapTransfer)
            .before(crate::interpreter::InterpreterStep),
    );
}

fn restore(
    mut commands: Commands,
    mut changes: MessageReader<MapRebuilt>,
    pending: Option<Res<Pending>>,
    data: Option<Res<MapData>>,
    vehicles: Option<ResMut<Vehicles>>,
) {
    if changes.read().count() == 0 {
        return;
    }
    let (Some(pending), Some(data), Some(mut vehicles)) = (pending, data, vehicles) else {
        return;
    };
    if data.map_id != pending.map_id {
        return;
    }
    vehicles.motion = pending.state.motion.clone().map(SavedMotion::into_motion);
    commands.remove_resource::<Pending>();
}
