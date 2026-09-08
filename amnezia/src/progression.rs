//! Persistent per-actor experience and the level it maps to. The battle system
//! reads a member's current level to build its stats from the actor's curve, and
//! adds the fight's reward experience on a win. Levels follow the RM2000 curve;
//! learned skills persist through level changes and scripted rewards.

use amnezia_data::ActorDef;
use bevy::prelude::*;
use std::collections::HashMap;

/// The RM2000 (RPG2000) experience ceiling: stored EXP never exceeds this
/// (EasyRPG `Game_Constants::MaxExpValue`).
const MAX_EXP: u32 = 999_999;

/// Stored total experience per actor id. An absent actor sits at the experience
/// of its starting level (see [`Progression::total`]).
#[derive(Resource, Default)]
pub struct Progression {
    experience: HashMap<u32, u32>,
    skills: HashMap<u32, Vec<u32>>,
}

impl Progression {
    /// The actor's stored total experience, defaulting to the experience needed
    /// to reach its starting level when it has earned none yet (so it never has
    /// to re-earn the levels it began the game with).
    pub fn total(&self, def: &ActorDef) -> u32 {
        self.experience
            .get(&def.id)
            .copied()
            .unwrap_or_else(|| exp_for_level(def.level, def))
    }

    /// Add `amount` experience to the actor, starting from its starting-level
    /// baseline the first time. The stored total is clamped to [`MAX_EXP`], as
    /// EasyRPG `Game_Actor::SetExp` clamps to the experience ceiling.
    pub fn add(&mut self, def: &ActorDef, amount: u32) {
        let previous_level = self.level(def);
        let known = self.known_skill_ids(def);
        let next = self.total(def).saturating_add(amount).min(MAX_EXP);
        self.experience.insert(def.id, next);
        self.learn_between(def, previous_level, known);
    }

    /// Known skills, including scripted changes. An actor without stored skills
    /// starts with the database learnings at or below its current level.
    pub fn known_skill_ids(&self, def: &ActorDef) -> Vec<u32> {
        if let Some(skills) = self.skills.get(&def.id) {
            return skills.clone();
        }
        let level = self.level(def);
        def.learnings
            .iter()
            .filter(|l| l.level <= level)
            .map(|l| l.skill_id)
            .collect()
    }

    /// Learn or forget a skill independently of the actor's level.
    pub fn change_skill(&mut self, def: &ActorDef, skill_id: u32, learn: bool) {
        let mut known = self.known_skill_ids(def);
        if learn {
            known.push(skill_id);
            known.sort_unstable();
            known.dedup();
        } else {
            known.retain(|&id| id != skill_id);
        }
        self.skills.insert(def.id, known);
    }

    fn learn_between(&mut self, def: &ActorDef, previous_level: u32, mut known: Vec<u32>) {
        let level = self.level(def);
        known.extend(
            def.learnings
                .iter()
                .filter(|l| l.level > previous_level && l.level <= level)
                .map(|l| l.skill_id),
        );
        known.sort_unstable();
        known.dedup();
        self.skills.insert(def.id, known);
    }

    /// The actor's current level: the highest level whose cumulative experience
    /// requirement its total meets, clamped to `[1, def.max_level]`.
    pub fn level(&self, def: &ActorDef) -> u32 {
        let total = self.total(def);
        let mut level = 1;
        while level < def.max_level && exp_for_level(level + 1, def) <= total {
            level += 1;
        }
        level
    }

    /// Set the actor's stored experience so its level becomes `target`, clamped to
    /// `[1, def.max_level]`. The
    /// total is set to exactly the target level's cumulative requirement, so the
    /// next fight's stats derive from the new level.
    pub fn set_level(&mut self, def: &ActorDef, target: u32) {
        let previous_level = self.level(def);
        let known = self.known_skill_ids(def);
        let clamped = target.clamp(1, def.max_level.max(1));
        self.experience.insert(def.id, exp_for_level(clamped, def));
        self.learn_between(def, previous_level, known);
    }

    /// Snapshot `(actor_id, exp)` pairs for the save file, in id order.
    pub fn entries(&self) -> Vec<(u32, u32)> {
        let mut entries = self
            .experience
            .iter()
            .map(|(&k, &v)| (k, v))
            .collect::<Vec<_>>();
        entries.sort_by_key(|&(k, _)| k);
        entries
    }

    /// Replace all stored experience from a loaded save.
    pub fn load(&mut self, entries: Vec<(u32, u32)>) {
        self.experience = entries.into_iter().collect();
        self.skills.clear();
    }

    pub fn skill_entries(&self) -> Vec<(u32, Vec<u32>)> {
        let mut entries = self
            .skills
            .iter()
            .map(|(&id, skills)| (id, skills.clone()))
            .collect::<Vec<_>>();
        entries.sort_by_key(|&(id, _)| id);
        entries
    }

    pub fn load_skills(&mut self, entries: Vec<(u32, Vec<u32>)>) {
        self.skills = entries.into_iter().collect();
    }
}

/// EasyRPG `Game_Actor::CalculateExp(level)` for the RPG2000 curve (`exp_curve
/// == 1`), reproduced exactly. Over `level` iterations it sums a running
/// `base + correction` term while `base` is scaled each step by an `inflation`
/// factor that itself decays toward 1; the decay rate depends on `level`, so it
/// is a constant for the whole call. The C++ accumulates in `int`, truncating
/// each `base + correction` toward zero — matched here by the `as i64` cast. The
/// return is the cumulative experience whose crossing raises the actor to level
/// `level + 1`, clamped to [`MAX_EXP`].
fn calculate_exp(level: u32, base: u32, inflation: u32, correction: u32) -> u32 {
    let mut result: i64 = 0;
    let mut base = base as f64;
    let correction = correction as f64;
    let mut inflation = 1.5 + inflation as f64 * 0.01;
    let decay = (level as f64 + 1.0) * 0.002 + 0.8;
    for _ in 0..level {
        result += (correction + base) as i64;
        base *= inflation;
        inflation = decay * (inflation - 1.0) + 1.0;
    }
    result.clamp(0, MAX_EXP as i64) as u32
}

/// Cumulative experience needed to REACH `level` (level 1 = 0). EasyRPG's
/// `GetBaseExp(level)` is `CalculateExp(level - 1)`, evaluated from the actor's
/// `exp_base`/`exp_inflation`/`exp_correction`. Monotonic in `level`, so more
/// experience always maps to an equal-or-higher level.
fn exp_for_level(level: u32, def: &ActorDef) -> u32 {
    if level <= 1 {
        return 0;
    }
    calculate_exp(
        level - 1,
        def.exp_base,
        def.exp_inflation,
        def.exp_correction,
    )
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
            learnings: Vec::new(),
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
    fn exp_curve_matches_the_rm2000_formula() {
        // The RM2000 default parameters (base 30, inflation 30, correction 0)
        // give this cumulative-to-reach table, hand-derived from EasyRPG's
        // `CalculateExp`: L1=0, L2=30, L3=84, L4=172.
        let d = def();
        assert_eq!(exp_for_level(1, &d), 0);
        assert_eq!(exp_for_level(2, &d), 30);
        assert_eq!(exp_for_level(3, &d), 84);
        assert_eq!(exp_for_level(4, &d), 172);

        // Correction is a flat per-step addend: with correction 100 the first
        // step (reaching level 2) costs base 30 + 100 = 130.
        let mut c = def();
        c.exp_correction = 100;
        assert_eq!(exp_for_level(2, &c), 130);

        // The curve is clamped to the RM2000 experience ceiling.
        let mut steep = def();
        steep.exp_base = 900_000;
        steep.exp_inflation = 100;
        steep.max_level = 50;
        assert_eq!(exp_for_level(50, &steep), MAX_EXP);
    }

    #[test]
    fn known_skills_are_the_learnings_at_or_below_the_level() {
        use amnezia_data::Learning;
        let mut d = def();
        d.max_level = 20;
        d.learnings = vec![
            Learning {
                level: 1,
                skill_id: 5,
            },
            Learning {
                level: 3,
                skill_id: 8,
            },
            Learning {
                level: 7,
                skill_id: 12,
            },
        ];
        let mut p = Progression::default();
        // At the starting level 1 only the level-1 skill is known.
        assert_eq!(p.known_skill_ids(&d), vec![5]);
        // Climbing to level 3 (its exp threshold) adds skill 8.
        p.add(&d, exp_for_level(3, &d));
        assert!(p.level(&d) >= 3);
        assert_eq!(p.known_skill_ids(&d), vec![5, 8]);
        // A level that crosses the level-7 learning adds skill 12.
        p.set_level(&d, 7);
        assert_eq!(p.known_skill_ids(&d), vec![5, 8, 12]);
    }

    #[test]
    fn stored_experience_is_clamped_to_the_ceiling() {
        let d = def();
        let mut p = Progression::default();
        p.add(&d, u32::MAX);
        assert_eq!(p.total(&d), MAX_EXP);
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
