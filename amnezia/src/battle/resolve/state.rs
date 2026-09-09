//! Per-turn status HP change (poison drain, regen) applied at a battler's turn
//! start, plus the top-of-round and on-damage status wear-off rolls.

use super::*;

impl Battle {
    /// Apply every active HP-changing state to `source` at the start of its turn
    /// (RM2000 timing, before it acts): a type-0 state drains and a type-1 state
    /// regenerates [`logic::state_hp_delta`]'s per-turn amount, a type-2 (and any
    /// unconfigured) state does nothing. A drain floors HP at 0 — from there the
    /// battler is defeated by the existing HP-based death path — and a regen caps
    /// at max HP. Each change is logged in the state-line style. Map/field HP
    /// change (`hp_change_map_*`) is deliberately skipped: it needs persistent
    /// overworld states, which battle state does not carry.
    pub(in crate::battle::resolve) fn tick_state_hp(&mut self, source: Source) {
        let (active, max_hp): (Vec<u32>, i32) = match source {
            Source::Party(i) => (
                self.members[i].states.iter().map(|&(id, _)| id).collect(),
                self.members[i].max_hp,
            ),
            Source::Enemy(i) => (
                self.enemies[i].states.iter().map(|&(id, _)| id).collect(),
                self.enemies[i].max_hp,
            ),
        };
        let mut changes: Vec<(i32, String)> = Vec::new();
        for sid in active {
            if let Some(def) = self.states.iter().find(|d| d.id == sid) {
                let delta = logic::state_hp_delta(def, max_hp);
                if delta != 0 {
                    changes.push((delta, def.name.clone()));
                }
            }
        }
        for (delta, state_name) in changes {
            let name = match source {
                Source::Party(i) => {
                    let f = &mut self.members[i];
                    f.hp = (f.hp + delta).clamp(0, max_hp);
                    f.name.clone()
                }
                Source::Enemy(i) => {
                    let e = &mut self.enemies[i];
                    e.hp = (e.hp + delta).clamp(0, max_hp);
                    e.name.clone()
                }
            };
            self.log.push(if delta < 0 {
                format!("{name}: {state_name} -{}", -delta)
            } else {
                format!("{name}: {state_name} +{delta}")
            });
        }
        if self.battler_hp(source) <= 0 {
            self.clear_battler_bonuses(source);
        }
        if let Source::Enemy(i) = source {
            self.start_foe_death(i, false);
        }
    }

    /// Wear off timed states at the top of a round: for every combatant advance
    /// each active state's held-turn count and roll its auto-release once past its
    /// hold turns (the death state is exempt), logging whatever lifts. Per-turn HP
    /// change (poison drain, regen) is applied separately, at the start of each
    /// battler's turn, by [`Battle::tick_state_hp`].
    pub(in crate::battle) fn run_recovery(&mut self) {
        let Battle {
            members,
            enemies,
            states,
            rng,
            log,
            ..
        } = self;
        let defs = states.as_slice();
        for f in members.iter_mut() {
            for id in logic::tick_recovery(&mut f.states, defs, || (rng_next(rng) % 100) as u32) {
                if let Some(d) = defs.iter().find(|d| d.id == id) {
                    log.push(format!("{}: {} elmúlt", f.name, d.name));
                }
            }
        }
        for e in enemies.iter_mut() {
            for id in logic::tick_recovery(&mut e.states, defs, || (rng_next(rng) % 100) as u32) {
                if let Some(d) = defs.iter().find(|d| d.id == id) {
                    log.push(format!("{}: {} elmúlt", e.name, d.name));
                }
            }
        }
    }

    /// Roll the damage wear-off of member `ti`'s active states after it is struck,
    /// logging whatever lifts.
    pub(in crate::battle::resolve) fn release_states_on_member(&mut self, ti: usize) {
        let Battle {
            members,
            states,
            rng,
            log,
            ..
        } = self;
        let defs = states.as_slice();
        let lifted = logic::release_on_damage(&mut members[ti].states, defs, || {
            (rng_next(rng) % 100) as u32
        });
        for id in lifted {
            if let Some(d) = defs.iter().find(|d| d.id == id) {
                log.push(format!("{}: {} elmúlt", members[ti].name, d.name));
            }
        }
    }

    /// Roll the damage wear-off of foe `ti`'s active states after it is struck,
    /// logging whatever lifts.
    pub(in crate::battle::resolve) fn release_states_on_enemy(&mut self, ti: usize) {
        let Battle {
            enemies,
            states,
            rng,
            log,
            ..
        } = self;
        let defs = states.as_slice();
        let lifted = logic::release_on_damage(&mut enemies[ti].states, defs, || {
            (rng_next(rng) % 100) as u32
        });
        for id in lifted {
            if let Some(d) = defs.iter().find(|d| d.id == id) {
                log.push(format!("{}: {} elmúlt", enemies[ti].name, d.name));
            }
        }
    }
}
