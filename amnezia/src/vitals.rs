//! Persistent HP/SP between battles. Missing entries mean full vitals at the actor's
//! current level; inns restore them by clearing stored damage.

use bevy::prelude::*;
use std::collections::HashMap;

/// Stored `(hp, sp)` per actor id. An absent actor is at full health.
#[derive(Resource, Default)]
pub struct Vitals {
    pools: HashMap<u32, (i32, i32)>,
    conditions: HashMap<u32, Vec<u32>>,
}

impl Vitals {
    pub fn set(&mut self, actor_id: u32, hp: i32, sp: i32) {
        self.pools.insert(actor_id, (hp.max(0), sp.max(0)));
        if hp <= 0 {
            self.conditions.remove(&actor_id);
        }
    }

    /// Clear stored damage so callers use full current-level vitals.
    pub fn heal_all(&mut self) {
        self.pools.clear();
        self.conditions.clear();
    }

    /// Restore one actor without changing anyone else's HP or SP.
    pub fn heal(&mut self, actor_id: u32) {
        self.pools.remove(&actor_id);
        self.conditions.remove(&actor_id);
    }

    /// The actor's stored `(hp, sp)`, or `None` when it has none yet — so a caller
    /// can default to full at the actor's current level rather than its
    /// starting-level HP/SP.
    pub fn get_stored(&self, actor_id: u32) -> Option<(i32, i32)> {
        self.pools.get(&actor_id).copied()
    }

    /// Snapshot `(actor_id, (hp, sp))` pairs for the save file, in id order.
    pub fn entries(&self) -> Vec<(u32, (i32, i32))> {
        let mut entries = self.pools.iter().map(|(&k, &v)| (k, v)).collect::<Vec<_>>();
        entries.sort_by_key(|&(k, _)| k);
        entries
    }

    /// Replace all stored vitals from a loaded save.
    pub fn load(&mut self, entries: Vec<(u32, (i32, i32))>) {
        self.pools = entries.into_iter().collect();
        self.conditions.clear();
    }

    pub fn states(&self, actor_id: u32) -> Vec<u32> {
        if self.get_stored(actor_id).is_some_and(|(hp, _)| hp == 0) {
            vec![1]
        } else {
            self.conditions.get(&actor_id).cloned().unwrap_or_default()
        }
    }

    pub fn set_states(&mut self, actor_id: u32, mut states: Vec<u32>) {
        states.retain(|id| *id > 1);
        states.sort_unstable();
        states.dedup();
        self.conditions.insert(actor_id, states);
    }

    pub fn change_condition(&mut self, actor_id: u32, state: u32, add: bool, full: (i32, i32)) {
        let (hp, sp) = self.get_stored(actor_id).unwrap_or(full);
        if state == 1 {
            if add {
                self.set(actor_id, 0, sp);
            } else if hp == 0 {
                self.set(actor_id, 1, sp);
            }
        } else if hp > 0 && state > 1 {
            let mut states = self.states(actor_id);
            if add {
                states.push(state);
            } else {
                states.retain(|id| *id != state);
            }
            self.set_states(actor_id, states);
        }
    }

    pub fn condition_entries(&self) -> Vec<(u32, Vec<u32>)> {
        let mut entries = self
            .conditions
            .iter()
            .filter(|(_, states)| !states.is_empty())
            .map(|(&id, states)| (id, states.clone()))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(id, _)| *id);
        entries
    }

    pub fn load_conditions(&mut self, entries: Vec<(u32, Vec<u32>)>) {
        self.conditions.clear();
        for (id, states) in entries {
            self.set_states(id, states);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_reads_back_vitals() {
        let mut vitals = Vitals::default();
        assert_eq!(vitals.get_stored(1), None);
        vitals.set(1, 20, 5);
        assert_eq!(vitals.get_stored(1), Some((20, 5)));
    }

    #[test]
    fn heal_all_clears_stored_damage() {
        let mut vitals = Vitals::default();
        vitals.set(1, 20, 5);
        vitals.heal_all();
        assert_eq!(vitals.get_stored(1), None);
    }

    #[test]
    fn entries_round_trip_through_load() {
        let mut vitals = Vitals::default();
        vitals.set(1, 40, 12);
        vitals.set(3, 30, 0);
        let snapshot = vitals.entries();
        let mut restored = Vitals::default();
        restored.load(snapshot);
        assert_eq!(restored.get_stored(1), Some((40, 12)));
        assert_eq!(restored.get_stored(3), Some((30, 0)));
    }
}
