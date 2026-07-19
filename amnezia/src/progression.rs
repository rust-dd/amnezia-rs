//! Persistent per-actor experience and the level it maps to. The battle system
//! reads a member's current level to build its stats from the actor's curve, and
//! adds the fight's reward experience on a win. Level rises with total experience
//! per the RM2000 (RPG2000) exp curve; it never drops below the actor's starting
//! level nor rises above its max level.

use amnezia_data::ActorDef;
use bevy::prelude::*;
use std::collections::HashMap;

/// Stored total experience per actor id. An absent actor sits at the experience
/// of its starting level (see [`Progression::total`]).
#[derive(Resource, Default)]
pub struct Progression(HashMap<u32, u32>);

impl Progression {
    /// The actor's stored total experience, defaulting to the experience needed
    /// to reach its starting level when it has earned none yet (so it never has
    /// to re-earn the levels it began the game with).
    pub fn total(&self, def: &ActorDef) -> u32 {
        self.0
            .get(&def.id)
            .copied()
            .unwrap_or_else(|| exp_for_level(def.level, def))
    }

    /// Add `amount` experience to the actor, starting from its starting-level
    /// baseline the first time.
    pub fn add(&mut self, def: &ActorDef, amount: u32) {
        let next = self.total(def).saturating_add(amount);
        self.0.insert(def.id, next);
    }

    /// The actor's current level: the highest level whose cumulative experience
    /// requirement its total meets, clamped to `[def.level, def.max_level]`.
    pub fn level(&self, def: &ActorDef) -> u32 {
        let total = self.total(def);
        let mut level = def.level.max(1);
        while level < def.max_level && exp_for_level(level + 1, def) <= total {
            level += 1;
        }
        level
    }

    /// Set the actor's stored experience so its level becomes `target`, clamped to
    /// `[def.level, def.max_level]` (RM2000 `ChangeLevel` clamps the same way). The
    /// total is set to exactly the target level's cumulative requirement, so the
    /// next fight's stats derive from the new level.
    pub fn set_level(&mut self, def: &ActorDef, target: u32) {
        let clamped = target.clamp(def.level.max(1), def.max_level);
        self.0.insert(def.id, exp_for_level(clamped, def));
    }

    /// Snapshot `(actor_id, exp)` pairs for the save file, in id order.
    pub fn entries(&self) -> Vec<(u32, u32)> {
        let mut entries: Vec<(u32, u32)> = self.0.iter().map(|(&k, &v)| (k, v)).collect();
        entries.sort_by_key(|&(k, _)| k);
        entries
    }

    /// Replace all stored experience from a loaded save.
    pub fn load(&mut self, entries: Vec<(u32, u32)>) {
        self.0 = entries.into_iter().collect();
    }
}

/// Cumulative experience needed to REACH `level` (level 1 = 0), from the actor's
/// RPG2000 exp-curve parameters: each step adds the running standard cost, which
/// then grows by `exp_inflation` percent plus a flat `exp_correction`. A
/// documented approximation of RM2000's exact rounding — monotonic, so more
/// experience always means an equal-or-higher level.
fn exp_for_level(level: u32, def: &ActorDef) -> u32 {
    if level <= 1 {
        return 0;
    }
    let factor = 1.0 + def.exp_inflation as f64 / 100.0;
    let correction = def.exp_correction as f64;
    let mut standard = def.exp_base as f64;
    let mut total = 0.0_f64;
    for _ in 1..level {
        total += standard.floor();
        standard = standard * factor + correction;
    }
    total as u32
}

pub struct ProgressionPlugin;

impl Plugin for ProgressionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Progression>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amnezia_data::ActorCurves;

    fn def() -> ActorDef {
        ActorDef {
            id: 1,
            name: "Ron".into(),
            title: String::new(),
            level: 1,
            max_level: 10,
            hp: 30,
            sp: 10,
            curves: ActorCurves::default(),
            exp_base: 30,
            exp_inflation: 30,
            exp_correction: 0,
            weapon: 0,
            shield: 0,
            armor: 0,
            helmet: 0,
            accessory: 0,
            two_weapons: false,
            fix_equipment: false,
            unarmed_animation: 0,
            face_name: String::new(),
            face_index: 0,
        }
    }

    #[test]
    fn exp_curve_is_zero_at_level_one_and_strictly_increasing() {
        let d = def();
        assert_eq!(exp_for_level(1, &d), 0);
        let mut prev = 0;
        for lvl in 2..=d.max_level {
            let e = exp_for_level(lvl, &d);
            assert!(e > prev, "exp must increase at level {lvl} ({e} !> {prev})");
            prev = e;
        }
    }

    #[test]
    fn earning_experience_raises_the_level() {
        let d = def();
        let mut p = Progression::default();
        assert_eq!(p.level(&d), 1);
        p.add(&d, exp_for_level(3, &d));
        assert!(
            p.level(&d) >= 3,
            "reaching level-3 exp should be at least level 3"
        );
    }

    #[test]
    fn level_is_clamped_to_the_starting_level_and_max() {
        let mut d = def();
        d.level = 5;
        let mut p = Progression::default();
        // No experience earned, but never below the starting level.
        assert_eq!(p.level(&d), 5);
        // A mountain of experience never exceeds max_level.
        p.add(&d, 10_000_000);
        assert_eq!(p.level(&d), d.max_level);
    }

    #[test]
    fn entries_round_trip_through_load() {
        let mut p = Progression::default();
        let d = def();
        p.add(&d, 500);
        let snap = p.entries();
        let mut q = Progression::default();
        q.load(snap);
        assert_eq!(q.total(&d), p.total(&d));
    }
}
