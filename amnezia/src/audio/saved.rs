use super::{AudioRequest, BgmTrack, CurrentBgm, MemorizedBgm};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct MusicState {
    pub current: Option<BgmTrack>,
    pub memorized: Option<BgmTrack>,
}

impl MusicState {
    pub(crate) fn valid(&self) -> bool {
        self.current.iter().chain(&self.memorized).all(|track| {
            track.volume.is_finite()
                && (0.0..=1.0).contains(&track.volume)
                && track.speed.is_finite()
                && track.speed > 0.0
                && track.fade_in.is_finite()
                && track.fade_in >= 0.0
        })
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Capture<'w> {
    current: Option<Res<'w, CurrentBgm>>,
    memorized: Option<Res<'w, MemorizedBgm>>,
}

impl Capture<'_> {
    pub(crate) fn snapshot(&self) -> Option<MusicState> {
        self.current.as_ref().map(|current| MusicState {
            current: current.track(),
            memorized: self.memorized.as_ref().and_then(|memory| memory.0.clone()),
        })
    }
}

#[derive(Resource)]
pub(crate) struct Pending {
    map_id: u32,
    arrived: bool,
    music: MusicState,
}

pub(crate) fn prepare(world: &mut World, map_id: u32, music: Option<MusicState>) {
    world.remove_resource::<Pending>();
    if let Some(mut memory) = world.get_resource_mut::<MemorizedBgm>() {
        memory.0 = None;
    }
    if let Some(music) = music {
        world.insert_resource(Pending {
            map_id,
            arrived: false,
            music,
        });
    } else if let Some(mut audio) = world.get_resource_mut::<Messages<AudioRequest>>() {
        audio.write(AudioRequest::StopBgm);
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_message::<AudioRequest>()
        .add_message::<crate::world::MapChanged>();
    crate::teleport::rebuild::register(app, crate::teleport::rebuild::Stage::SavedMusic, restore);
}

fn restore(
    mut commands: Commands,
    mut pending: Option<ResMut<Pending>>,
    map: Option<Res<crate::world::MapData>>,
    title: Option<Res<crate::title::TitleActive>>,
    mut changed: MessageReader<crate::world::MapChanged>,
    mut audio: MessageWriter<AudioRequest>,
) {
    let arrived = changed.read().count() != 0;
    let Some(ref mut pending) = pending else {
        return;
    };
    pending.arrived |= arrived && map.as_ref().is_some_and(|map| map.map_id == pending.map_id);
    if !pending.arrived || title.is_some_and(|title| title.0) {
        return;
    }
    audio.write(AudioRequest::StopBgm);
    if let Some(track) = &pending.music.current {
        audio.write(track.replay());
    }
    commands.insert_resource(MemorizedBgm(pending.music.memorized.clone()));
    commands.remove_resource::<Pending>();
}
