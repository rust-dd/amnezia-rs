//! Persistent party vitals: each actor's current HP/SP carried between battles
//! and healed at an inn. The battle system reads a stored entry when building a
//! fighter (an absent entry means the actor starts the fight full at its current
//! level) and writes it back when the fight ends.

use bevy::prelude::*;
use std::collections::HashMap;

/// Stored `(hp, sp)` per actor id. An absent actor is at full health.
#[derive(Resource, Default)]
pub struct Vitals(HashMap<u32, (i32, i32)>);

impl Vitals {
    /// Store an actor's `(hp, sp)` after a battle.
    pub fn set(&mut self, actor_id: u32, hp: i32, sp: i32) {
        self.0.insert(actor_id, (hp, sp));
    }

    /// Restore every party member to full HP/SP by dropping all stored damage,
    /// so each actor's `get()` returns its full ActorDef values again.
    pub fn heal_all(&mut self) {
        self.0.clear();
    }

    /// Restore one actor without changing anyone else's HP or SP.
    pub fn heal(&mut self, actor_id: u32) {
        self.0.remove(&actor_id);
    }

    /// The actor's stored `(hp, sp)`, or `None` when it has none yet — so a caller
    /// can default to full at the actor's current level rather than its
    /// starting-level HP/SP.
    pub fn get_stored(&self, actor_id: u32) -> Option<(i32, i32)> {
        self.0.get(&actor_id).copied()
    }

    /// Snapshot `(actor_id, (hp, sp))` pairs for the save file, in id order.
    pub fn entries(&self) -> Vec<(u32, (i32, i32))> {
        let mut entries: Vec<(u32, (i32, i32))> = self.0.iter().map(|(&k, &v)| (k, v)).collect();
        entries.sort_by_key(|&(k, _)| k);
        entries
    }

    /// Replace all stored vitals from a loaded save.
    pub fn load(&mut self, entries: Vec<(u32, (i32, i32))>) {
        self.0 = entries.into_iter().collect();
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
