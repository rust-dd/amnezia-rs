use super::SaveGame;
use crate::gamedata::GameData;
use std::collections::HashSet;

fn identifiers(values: &mut Vec<u32>, exists: impl Fn(u32) -> bool) -> bool {
    let before = values.len();
    let mut seen = HashSet::new();
    values.retain(|&id| exists(id) && seen.insert(id));
    values.len() == before
}

fn entries<T>(values: &mut Vec<(u32, T)>, exists: impl Fn(u32) -> bool) -> bool {
    let before = values.len();
    let mut seen = HashSet::new();
    // Last-value wins matches the runtime resources' HashMap collection.
    values.reverse();
    values.retain(|(id, _)| exists(*id) && seen.insert(*id));
    values.reverse();
    values.len() == before
}

fn equipment(entries: &mut [(u32, [u32; 5])], data: &GameData) -> bool {
    let mut valid = true;
    for (actor_id, slots) in entries {
        let actor = data.actor(*actor_id).unwrap();
        for (slot, item_id) in slots.iter_mut().enumerate() {
            if *item_id != 0
                && !data
                    .item(*item_id)
                    .is_some_and(|item| crate::equipment::can_equip(actor, slot, item))
            {
                *item_id = 0;
                valid = false;
            }
        }
    }
    valid
}

pub(super) fn prepare(game: &mut SaveGame, data: Option<&GameData>) -> bool {
    let actor_exists = |id| id > 0 && data.is_none_or(|data| data.actor(id).is_some());
    let mut valid = identifiers(&mut game.party, actor_exists);
    if game.party.len() > crate::state::MAX_PARTY {
        game.party.truncate(crate::state::MAX_PARTY);
        valid = false;
    }
    if game.party.is_empty() {
        return false;
    }
    valid &= entries(&mut game.switches, |id| id > 0);
    valid &= entries(&mut game.variables, |id| id > 0);
    valid &= entries(&mut game.items, |id| {
        id > 0 && data.is_none_or(|data| data.item(id).is_some())
    });
    valid &= entries(&mut game.progression, actor_exists);
    valid &= entries(&mut game.vitals, actor_exists);
    valid &= entries(&mut game.equipment, actor_exists);
    valid &= entries(&mut game.learned_skills, actor_exists);
    valid &= entries(&mut game.conditions, actor_exists);
    for (_, skills) in &mut game.learned_skills {
        valid &= identifiers(skills, |id| {
            id > 0 && data.is_none_or(|data| data.skills.iter().any(|skill| skill.id == id))
        });
        skills.sort_unstable();
    }
    for (_, states) in &mut game.conditions {
        valid &= identifiers(states, |id| {
            id > 1
                && crate::conditions::definitions()
                    .iter()
                    .any(|state| state.id == id)
        });
        states.sort_unstable();
    }
    if let Some(data) = data {
        valid &= equipment(&mut game.equipment, data);
    }
    valid || game.format_version < 15
}
