//! A slain foe's death-out: starting the blink-and-fade (or explosion) and
//! advancing it while resolution holds for the beat to be seen.

use super::*;

impl Battle {
    /// Start foe `ti`'s death-out if it was just reduced to 0 HP and isn't already
    /// leaving: a zoom-fade explosion for a self-destruct (`explode`), else the
    /// RM2000 blink-and-fade. A fled foe (it kept its HP) and an already-dying foe
    /// are left alone. `resolve_tick` then holds until it elapses.
    pub(in crate::battle::resolve) fn start_foe_death(&mut self, ti: usize, explode: bool) {
        let foe = &mut self.enemies[ti];
        if foe.hp <= 0 && !foe.fled && foe.dying.is_none() {
            foe.dying = Some(Dying {
                elapsed: 0.0,
                secs: if explode { EXPLODE_SECS } else { DEATH_SECS },
                explode,
            });
            // The kill SE fires once, guarded by the same transition that starts
            // the death-out (EasyRPG `ProcessBattleActionDeath`).
            self.pending_se.push(BattleSe::EnemyDefeated);
        }
    }

    /// Whether any foe is mid death-out; `resolve_tick` holds the step while so.
    pub(in crate::battle) fn death_in_progress(&self) -> bool {
        self.enemies
            .iter()
            .any(|e| e.dying.as_ref().is_some_and(|d| d.elapsed < d.secs))
    }

    /// Advance every in-progress death-out by `dt` real seconds (driven each frame
    /// by `battle::scene`), clamped at its length so [`Battle::death_in_progress`]
    /// eventually clears and resolution resumes.
    pub(in crate::battle) fn advance_deaths(&mut self, dt: f32) {
        for foe in &mut self.enemies {
            if let Some(d) = &mut foe.dying
                && d.elapsed < d.secs
            {
                d.elapsed = (d.elapsed + dt).min(d.secs);
            }
        }
    }
}
