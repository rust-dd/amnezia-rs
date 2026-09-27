//! Map background music: each map's BGM plays when the map becomes active — its
//! initial load once the title releases the world, and every teleport arrival —
//! resolving the LMT `music_type` inheritance the converter carried into
//! `map_info.ron`. Mirrors RM2000 `Game_Map::PlayBgm`: a type-2 map requests its
//! own track, while a type-1 (event-controlled) map, or one inheriting up to a
//! silent/rootless owner, leaves the current BGM alone — so the intro's own
//! `PlayBGM`, and a same-map teleport, are never interrupted.

use crate::assets::{asset_root, load_ron};
use crate::audio::AudioRequest;
use crate::title::TitleActive;
use crate::world::MapData;
use amnezia_data::{MapBgm, MapInfoDef, resolve_map_bgm};
use bevy::prelude::*;

/// The LMT map-info tree (each map's parent, `music_type`, and track), loaded
/// once from `map_info.ron`. The game resolves a map's effective BGM from it.
#[derive(Resource)]
pub struct MapInfoData(pub Vec<MapInfoDef>);

impl MapInfoData {
    /// The effective BGM for map `map_id`, walking the inheritance chain.
    fn resolve(&self, map_id: u32) -> MapBgm {
        resolve_map_bgm(&self.0, map_id)
    }
}

pub struct MapBgmPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MapMusic;

impl Plugin for MapBgmPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MapInfoData(load_ron(&format!(
            "{}/map_info.ron",
            asset_root()
        ))))
        .add_systems(
            Update,
            flush
                .in_set(MapMusic)
                .after(crate::teleport::MapTransfer)
                .before(crate::interpreter::ParallelStep)
                .before(crate::interpreter::InterpreterStep),
        );
    }
}

pub(crate) fn flush(world: &mut World) {
    if world.contains_resource::<MapInfoData>() {
        world.run_system_cached(play_map_bgm).unwrap();
    }
}

/// Map entry plays once after the title releases the world. Save restoration
/// retains its recorded track instead of replaying the map's initial music.
fn play_map_bgm(
    map_data: Res<MapData>,
    map_info: Res<MapInfoData>,
    title: Res<TitleActive>,
    restore: Option<Res<crate::audio::saved::Pending>>,
    mut audio: MessageWriter<AudioRequest>,
    mut pending: Local<bool>,
) {
    if title.0 {
        *pending = true;
        return;
    }
    if restore.is_some() {
        *pending = false;
        return;
    }
    if !map_data.is_changed() && !*pending {
        return;
    }
    *pending = false;
    let resolved = map_info.resolve(map_data.map_id);
    match &resolved {
        MapBgm::Play(m) => eprintln!("[MAPBGM] map {} -> play '{}'", map_data.map_id, m.name),
        _ => eprintln!("[MAPBGM] map {} -> leave current BGM", map_data.map_id),
    }
    if let MapBgm::Play(music) = resolved {
        audio.write(AudioRequest::from_music(&music));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amnezia_data::MusicDef;

    fn map(id: u32, music_type: u32, name: &str) -> MapInfoDef {
        MapInfoDef {
            id,
            parent: 0,
            music_type,
            music: MusicDef {
                name: name.to_string(),
                volume: 80,
                tempo: 100,
                balance: 50,
                fadein: 0,
            },
        }
    }

    fn app_on_map(map_id: u32, maps: Vec<MapInfoDef>, title: bool) -> App {
        let mut data = MapData::for_test(10, 10);
        data.map_id = map_id;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<AudioRequest>()
            .insert_resource(MapInfoData(maps))
            .insert_resource(TitleActive(title))
            .insert_resource(data)
            .add_systems(Update, play_map_bgm);
        app
    }

    fn requests(app: &App) -> Vec<AudioRequest> {
        let messages = app.world().resource::<Messages<AudioRequest>>();
        let mut cursor = messages.get_cursor();
        cursor.read(messages).cloned().collect()
    }

    #[test]
    fn entering_a_type_two_map_requests_its_bgm() {
        let mut app = app_on_map(5, vec![map(5, 2, "Town")], false);
        app.update();
        let sent = requests(&app);
        assert!(
            sent.iter()
                .any(|r| matches!(r, AudioRequest::Bgm { name, .. } if name == "Town")),
            "a type-2 map must request its own track: {sent:?}"
        );
    }

    #[test]
    fn entering_a_type_one_map_leaves_the_bgm_unchanged() {
        // A type-1 (event-controlled) map emits no request, so whatever the intro
        // or a prior map set keeps playing.
        let mut app = app_on_map(5, vec![map(5, 1, "Ignored")], false);
        app.update();
        assert!(
            requests(&app).is_empty(),
            "a type-1 map must not touch the BGM"
        );
    }

    #[test]
    fn the_title_defers_the_map_bgm_until_it_releases_the_world() {
        let mut app = app_on_map(5, vec![map(5, 2, "Town")], true);
        app.update();
        assert!(
            requests(&app).is_empty(),
            "no map BGM while the title theme owns the screen"
        );
        app.insert_resource(TitleActive(false));
        app.update();
        let sent = requests(&app);
        assert!(
            sent.iter()
                .any(|r| matches!(r, AudioRequest::Bgm { name, .. } if name == "Town")),
            "the deferred map BGM must play once the title releases the world: {sent:?}"
        );
    }
}
