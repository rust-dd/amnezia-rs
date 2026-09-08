use amnezia_data::MusicDef;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Resource, Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SystemBgm(BTreeMap<u32, MusicDef>);

impl SystemBgm {
    pub fn change(&mut self, name: &str, params: &[i32]) {
        let [slot, fadein, volume, tempo, balance, ..] = params else {
            return;
        };
        if !(0..=6).contains(slot) {
            return;
        }
        self.0.insert(
            *slot as u32,
            MusicDef {
                name: name.into(),
                fadein: (*fadein).max(0) as u32,
                volume: (*volume).clamp(0, 100) as u32,
                tempo: (*tempo).clamp(50, 200) as u32,
                balance: (*balance).clamp(0, 100) as u32,
            },
        );
    }

    pub fn get<'a>(&'a self, slot: u32, fallback: &'a MusicDef) -> &'a MusicDef {
        self.0.get(&slot).unwrap_or(fallback)
    }
}

pub fn resolve<'a>(
    music: Option<&'a SystemBgm>,
    slot: u32,
    fallback: &'a MusicDef,
) -> &'a MusicDef {
    music.map_or(fallback, |music| music.get(slot, fallback))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_and_airship_music_override_only_their_scene_and_survive_save() {
        let mut music = SystemBgm::default();
        music.change("Boss", &[0, 500, 90, 100, 50]);
        music.change("Fortress", &[5, 0, 90, 100, 50]);
        let fallback = MusicDef::default();
        assert_eq!(music.get(0, &fallback).name, "Boss");
        assert_eq!(music.get(5, &fallback).name, "Fortress");
        assert_eq!(music.get(1, &fallback), &fallback);
        let restored = ron::from_str::<SystemBgm>(&ron::to_string(&music).unwrap()).unwrap();
        assert_eq!(restored, music);
    }
}
