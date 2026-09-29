//! Death and explosion visuals advance before the action timeline each update.

use super::*;

impl Battle {
    /// Emit the collapse sound once, on the live-to-dying transition.
    pub(in crate::battle) fn start_foe_death(&mut self, ti: usize, explode: bool) {
        if self.enemies[ti].hp <= 0 {
            self.mark_knocked_out(Source::Enemy(ti));
        }
        let foe = &mut self.enemies[ti];
        if foe.hp <= 0 && !foe.fled && foe.dying.is_none() {
            foe.dying = Some(Dying {
                elapsed: 0.0,
                secs: if explode { EXPLODE_SECS } else { DEATH_SECS },
                explode,
            });
            self.pending_se.push(BattleSe::EnemyDefeated);
        }
    }

    pub(in crate::battle) fn death_in_progress(&self) -> bool {
        self.enemies
            .iter()
            .any(|e| e.dying.as_ref().is_some_and(|d| d.elapsed < d.secs))
    }

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
