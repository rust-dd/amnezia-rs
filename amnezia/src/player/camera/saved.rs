use super::CameraPan;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct CameraState {
    pub offset: [f32; 2],
    pub target: [f32; 2],
    pub speed: f32,
    pub locked: bool,
    pub position: Option<[f32; 2]>,
    /// Unwrapped tracking position keeps the next follow step continuous on looping maps.
    pub previous_player: Option<[f32; 2]>,
    #[serde(default)]
    pub(super) tracking: Option<super::tracking::Tracking>,
}

impl CameraPan {
    pub(crate) fn snapshot(&self) -> CameraState {
        CameraState {
            offset: self.offset.to_array(),
            target: self.target.to_array(),
            speed: self.speed,
            locked: self.locked,
            position: self.position.map(|value| value.to_array()),
            previous_player: self.previous_player.map(|value| value.to_array()),
            tracking: self.tracking.clone(),
        }
    }
}

impl CameraState {
    pub(crate) fn valid(&self) -> bool {
        self.speed.is_finite()
            && self
                .tracking
                .as_ref()
                .is_none_or(super::tracking::Tracking::valid)
            && self.speed > 0.0
            && self
                .offset
                .iter()
                .chain(&self.target)
                .chain(self.position.iter().flatten())
                .chain(self.previous_player.iter().flatten())
                .all(|value| value.is_finite())
    }

    pub(crate) fn into_pan(self) -> CameraPan {
        CameraPan {
            offset: Vec2::from_array(self.offset),
            target: Vec2::from_array(self.target),
            speed: self.speed,
            locked: self.locked,
            position: self.position.map(Vec2::from_array),
            previous_player: self.previous_player.map(Vec2::from_array),
            tracking: self.tracking,
        }
    }
}

#[derive(Resource)]
pub(crate) struct Pending {
    map_id: u32,
    camera: CameraState,
}

pub(crate) fn prepare(world: &mut World, map_id: u32, camera: Option<CameraState>) {
    world.remove_resource::<Pending>();
    if let Some(camera) = camera {
        world.insert_resource(Pending { map_id, camera });
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_message::<crate::world::MapChanged>().add_systems(
        Update,
        restore
            .after(crate::teleport::MapTransfer)
            .before(super::CameraFollow)
            .before(crate::interpreter::InterpreterStep),
    );
}

fn restore(
    mut commands: Commands,
    pending: Option<Res<Pending>>,
    map: Option<Res<crate::world::MapData>>,
    mut changed: MessageReader<crate::world::MapChanged>,
) {
    if changed.read().count() == 0 {
        return;
    }
    let Some(pending) = pending else {
        return;
    };
    if map.as_ref().is_none_or(|map| map.map_id != pending.map_id) {
        return;
    }
    commands.insert_resource(pending.camera.clone().into_pan());
    commands.remove_resource::<Pending>();
}
