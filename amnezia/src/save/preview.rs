use super::{SAVE_FORMAT_VERSION, identities, numeric, slots, storage};
use crate::gamedata::GameData;
use crate::progression::Progression;
use std::path::Path;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PartyPreview {
    pub name: String,
    pub level: u32,
    pub hp: i32,
    pub faces: Vec<(String, u32)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Contents {
    Empty,
    Corrupt,
    Party(PartyPreview),
}

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub contents: Contents,
    pub timestamp: Option<Duration>,
}

pub(crate) fn catalog(first: &Path, data: &GameData) -> Vec<Entry> {
    (1..=slots::COUNT)
        .map(|number| read(&slots::ActiveSlot::new(number).unwrap().path(first), data))
        .collect()
}

pub(crate) fn latest(entries: &[Entry]) -> usize {
    let mut latest = None;
    let mut selected = 0;
    for (index, entry) in entries.iter().enumerate() {
        if let Some(time) = entry.timestamp
            && matches!(entry.contents, Contents::Party(_))
            && latest.is_none_or(|previous| time > previous)
        {
            latest = Some(time);
            selected = index;
        }
    }
    selected
}

fn read(path: &Path, data: &GameData) -> Entry {
    let source = storage::source_path(path);
    let metadata = std::fs::metadata(&source).ok();
    let timestamp = metadata
        .as_ref()
        .and_then(|value| value.modified().ok())
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok());
    let Some(mut game) = storage::read_save(path) else {
        return Entry {
            contents: if metadata.is_some() {
                Contents::Corrupt
            } else {
                Contents::Empty
            },
            timestamp,
        };
    };
    if game.format_version > SAVE_FORMAT_VERSION
        || !identities::prepare(&mut game, Some(data))
        || !numeric::prepare(&mut game, Some(data))
    {
        return Entry {
            contents: Contents::Corrupt,
            timestamp,
        };
    }
    let actor = data.actor(game.party[0]).unwrap();
    let mut progression = Progression::default();
    progression.load(game.progression);
    let level = progression.level(actor);
    let hp = game
        .vitals
        .iter()
        .find(|(id, _)| *id == actor.id)
        .map(|(_, (hp, _))| *hp)
        .unwrap_or_else(|| {
            crate::battle::logic::actor_hp_sp_at(&actor.curves, level, actor.hp, actor.sp).0 as i32
        });
    let name = if actor.id == 1 && (game.format_version > 0 || !game.hero_name.is_empty()) {
        game.hero_name
    } else {
        actor.name.clone()
    };
    let faces = game
        .party
        .iter()
        .filter_map(|&id| data.actor(id))
        .map(|actor| (actor.face_name.clone(), actor.face_index))
        .collect();
    Entry {
        contents: Contents::Party(PartyPreview {
            name,
            level,
            hp,
            faces,
        }),
        timestamp: game.saved_at.map(Duration::from_secs).or(timestamp),
    }
}

#[cfg(test)]
mod tests;
