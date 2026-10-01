//! Shared LMT music-inheritance resolver, matching RM2000 `Game_Map::PlayBgm`.

use serde::{Deserialize, Serialize};

use crate::MusicDef;

/// Map-info node: parent 0 is the root. `music_type` selects 0 = inherit,
/// 1 = keep current (event-controlled), 2 = play this map's track.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapInfoDef {
    pub id: u32,
    pub parent: u32,
    pub music_type: u32,
    #[serde(default)]
    pub music: MusicDef,
}

/// The BGM action a map load resolves to after walking the `music_type == 0`
/// inheritance chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapBgm {
    /// Play the resolved track; `(OFF)` tells the audio layer to stop.
    Play(MusicDef),
    /// Keep current BGM for event-controlled maps, missing tracks or unresolved ancestry.
    Keep,
}

/// Follow type-0 parent links to a music owner; invalid or cyclic ancestry keeps the BGM.
pub fn resolve_map_bgm(maps: &[MapInfoDef], map_id: u32) -> MapBgm {
    let Some(mut current) = maps.iter().find(|m| m.id == map_id) else {
        return MapBgm::Keep;
    };
    // Bound traversal in case the imported parent links form a cycle.
    let mut steps = maps.len();
    while current.music_type == 0 {
        if current.parent == 0 || current.parent == current.id {
            return MapBgm::Keep;
        }
        let Some(parent) = maps.iter().find(|m| m.id == current.parent) else {
            return MapBgm::Keep;
        };
        current = parent;
        steps = steps.saturating_sub(1);
        if steps == 0 {
            return MapBgm::Keep;
        }
    }
    if current.music_type == 1 || current.music.name.is_empty() {
        return MapBgm::Keep;
    }
    MapBgm::Play(current.music.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn music(name: &str) -> MusicDef {
        MusicDef {
            name: name.to_string(),
            volume: 90,
            tempo: 100,
            balance: 50,
            fadein: 0,
        }
    }

    fn map(id: u32, parent: u32, music_type: u32, name: &str) -> MapInfoDef {
        MapInfoDef {
            id,
            parent,
            music_type,
            music: music(name),
        }
    }

    #[test]
    fn type_two_map_plays_its_own_track() {
        let maps = vec![map(1, 0, 2, "Town")];
        assert_eq!(resolve_map_bgm(&maps, 1), MapBgm::Play(music("Town")));
    }

    #[test]
    fn type_one_map_keeps_the_current_bgm() {
        let maps = vec![map(1, 0, 1, "Ignored")];
        assert_eq!(resolve_map_bgm(&maps, 1), MapBgm::Keep);
    }

    #[test]
    fn type_zero_walks_up_to_a_type_two_parent() {
        let maps = vec![map(1, 0, 2, "Town"), map(2, 1, 0, ""), map(3, 2, 0, "")];
        assert_eq!(resolve_map_bgm(&maps, 3), MapBgm::Play(music("Town")));
    }

    #[test]
    fn type_zero_stopping_at_a_type_one_parent_keeps_current() {
        let maps = vec![map(1, 0, 1, "X"), map(2, 1, 0, "")];
        assert_eq!(resolve_map_bgm(&maps, 2), MapBgm::Keep);
    }

    #[test]
    fn inheriting_up_to_the_root_keeps_current() {
        let maps = vec![map(1, 0, 0, "")];
        assert_eq!(resolve_map_bgm(&maps, 1), MapBgm::Keep);
    }

    #[test]
    fn unknown_map_keeps_current() {
        let maps = vec![map(1, 0, 2, "Town")];
        assert_eq!(resolve_map_bgm(&maps, 99), MapBgm::Keep);
    }

    #[test]
    fn a_parent_cycle_resolves_to_keep_without_hanging() {
        let maps = vec![map(1, 2, 0, ""), map(2, 1, 0, "")];
        assert_eq!(resolve_map_bgm(&maps, 1), MapBgm::Keep);
    }

    #[test]
    fn a_type_two_map_with_off_name_is_carried_through() {
        // `(OFF)` must stop the prior track, not leave it playing via Keep.
        let maps = vec![map(1, 0, 2, "(OFF)")];
        assert_eq!(resolve_map_bgm(&maps, 1), MapBgm::Play(music("(OFF)")));
    }
}
