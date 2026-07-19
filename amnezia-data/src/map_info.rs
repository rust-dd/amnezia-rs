//! The LMT map-info tree the game resolves each map's background music from.
//! The converter parses it offline into `map_info.ron` and the game reads it
//! back; the resolver here is the single source of truth both the converter
//! tests and the game share, mirroring RM2000's `Game_Map::PlayBgm`.

use serde::{Deserialize, Serialize};

use crate::MusicDef;

/// One node of the map-info tree: the map `id`, its `parent` map id (0 = the
/// tree root), the `music_type`, and the map's own `music` track. `music_type`
/// mirrors RM2000: 0 = inherit the parent's music, 1 = keep the currently
/// playing (event-controlled) BGM, 2 = play `music`.
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
    /// Play this specific track — a `music_type == 2` map, or the type-2 ancestor
    /// a chain of type-0 maps inherits from. An `(OFF)`/empty track name is
    /// carried through for the audio layer to turn into a stop.
    Play(MusicDef),
    /// Leave the current BGM untouched: an event-controlled map (`music_type == 1`),
    /// one that inherits up to the root with no owner, one that names no track, or
    /// an unknown id.
    Keep,
}

/// Resolve the effective BGM for map `map_id` from the map-info tree, mirroring
/// RM2000's `Game_Map::PlayBgm`: start at the map and walk `parent` links while
/// `music_type == 0`, stopping at the first type-1/2 node or the tree root, then
/// decide. A type-2 node with a named track plays it; a type-1 node, the root, a
/// node with no track name, or an unknown id keeps whatever is already playing.
pub fn resolve_map_bgm(maps: &[MapInfoDef], map_id: u32) -> MapBgm {
    let Some(mut current) = maps.iter().find(|m| m.id == map_id) else {
        return MapBgm::Keep;
    };
    // The tree is shallow, but guard against a malformed parent cycle regardless.
    let mut steps = maps.len();
    while current.music_type == 0 {
        // Parent 0 is the tree root and a self-parent is degenerate: either ends
        // the walk with no music owner, so the current BGM stays.
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
        // 3 inherits from 2 inherits from 1 (a type-2 town): the walk climbs both
        // type-0 links and plays the town's track.
        let maps = vec![map(1, 0, 2, "Town"), map(2, 1, 0, ""), map(3, 2, 0, "")];
        assert_eq!(resolve_map_bgm(&maps, 3), MapBgm::Play(music("Town")));
    }

    #[test]
    fn type_zero_stopping_at_a_type_one_parent_keeps_current() {
        // A type-0 child of a type-1 (event-controlled) parent keeps the BGM.
        let maps = vec![map(1, 0, 1, "X"), map(2, 1, 0, "")];
        assert_eq!(resolve_map_bgm(&maps, 2), MapBgm::Keep);
    }

    #[test]
    fn inheriting_up_to_the_root_keeps_current() {
        // A lone type-0 map whose parent is the root (0) has no music owner.
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
        // Two type-0 maps pointing at each other: the step guard breaks the cycle.
        let maps = vec![map(1, 2, 0, ""), map(2, 1, 0, "")];
        assert_eq!(resolve_map_bgm(&maps, 1), MapBgm::Keep);
    }

    #[test]
    fn a_type_two_map_with_off_name_is_carried_through() {
        // "(OFF)" is a real, non-empty name: the audio layer maps it to a stop, so
        // it must resolve to Play (not Keep, which would leave the prior music on).
        let maps = vec![map(1, 0, 2, "(OFF)")];
        assert_eq!(resolve_map_bgm(&maps, 1), MapBgm::Play(music("(OFF)")));
    }
}
