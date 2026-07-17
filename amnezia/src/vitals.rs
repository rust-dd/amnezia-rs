//! Persistent party vitals: each actor's current HP/SP carried between battles
//! (and, later, healed at an inn). The battle system reads them when building a
//! fighter and writes them back when the fight ends; before an actor's first
//! battle its entry defaults to the full HP/SP from its [`ActorDef`].

use amnezia_data::ActorDef;
use bevy::prelude::*;
use std::collections::HashMap;

/// Stored `(hp, sp)` per actor id. An absent actor is at full health.
#[derive(Resource, Default)]
pub struct Vitals(HashMap<u32, (i32, i32)>);

impl Vitals {
    /// The actor's stored `(hp, sp)`, or its full starting values when it has not
    /// fought yet.
    pub fn get(&self, actor: &ActorDef) -> (i32, i32) {
        self.0
            .get(&actor.id)
            .copied()
            .unwrap_or((actor.hp as i32, actor.sp as i32))
    }

    /// Store an actor's `(hp, sp)` after a battle.
    pub fn set(&mut self, actor_id: u32, hp: i32, sp: i32) {
        self.0.insert(actor_id, (hp, sp));
    }

    /// Restore every party member to full HP/SP by dropping all stored damage,
    /// so each actor's `get()` returns its full ActorDef values again.
    pub fn heal_all(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor() -> ActorDef {
        ActorDef {
            id: 1,
            name: "Ron".into(),
            title: "Zsoldos".into(),
            level: 2,
            max_level: 50,
            hp: 63,
            sp: 37,
        }
    }

    #[test]
    fn defaults_to_full_then_persists_stored_values() {
        let mut vitals = Vitals::default();
        assert_eq!(vitals.get(&actor()), (63, 37));
        vitals.set(1, 20, 5);
        assert_eq!(vitals.get(&actor()), (20, 5));
    }

    #[test]
    fn heal_all_restores_full_values() {
        let mut vitals = Vitals::default();
        vitals.set(1, 20, 5);
        vitals.heal_all();
        let actor = actor();
        assert_eq!(vitals.get(&actor), (actor.hp as i32, actor.sp as i32));
    }
}
