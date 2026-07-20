//! Ending the fight: the victory/defeat end check, the troop reward tally, the
//! flee roll, and the concluding message run staged into the log.

use super::*;
use crate::i18n;

impl Battle {
    /// The terminal outcome, if reached: victory when every enemy is down, defeat
    /// when every member is. `None` while the fight continues.
    pub fn end_state(&self) -> Option<BattleOutcome> {
        if self.enemies.iter().all(|e| !e.alive()) {
            Some(BattleOutcome::Victory)
        } else if self.members.iter().all(|m| !m.alive()) {
            Some(BattleOutcome::Defeat)
        } else {
            None
        }
    }

    /// Total `(exp, gold)` for defeating the troop: every foe actually beaten. A
    /// fled foe (RM2000 Escape) is not defeated, so it grants nothing.
    pub fn victory_rewards(&self) -> (u32, u32) {
        let rewards: Vec<(u32, u32)> = self
            .enemies
            .iter()
            .filter(|e| !e.fled)
            .map(|e| (e.exp, e.gold))
            .collect();
        logic::total_rewards(&rewards)
    }

    /// Roll a party escape against the persisted RM2000 escape chance (EasyRPG
    /// `TryEscape`): a first strike escapes outright; otherwise a `PercentChance`
    /// draw against `escape_chance`. On failure the chance is raised by 10 for the
    /// next attempt and `false` is returned, so the caller forfeits the party's
    /// turn. The chance was fixed at [`Battle::build`] from the two sides' average
    /// agilities and is never recomputed here.
    pub fn attempt_escape(&mut self) -> bool {
        if self.first_strike {
            return true;
        }
        let roll = (rng_next(&mut self.rng) % 100) as u32;
        if logic::escape_succeeds(self.escape_chance, roll) {
            true
        } else {
            self.escape_chance += 10;
            false
        }
    }

    /// Conclude the fight with `outcome` and wait in the outcome phase for the
    /// player. On victory this records the reward and stages the RM2000
    /// `ProcessSceneActionVictory` message run as separate log lines — the victory
    /// term, then the experience and gold gained. The reward is *paid* (gold added,
    /// experience awarded, level-ups appended) by `battle::apply_victory_rewards`
    /// once the outcome is entered, since that needs the party's progression and
    /// inventory resources.
    pub fn finish(&mut self, outcome: BattleOutcome) {
        self.outcome = Some(outcome);
        self.phase = Phase::Outcome;
        match outcome {
            BattleOutcome::Victory => {
                let (exp, gold) = self.victory_rewards();
                self.reward_exp = exp;
                self.reward_gold = gold;
                self.log.push(i18n::tr(&self.text.victory));
                if exp > 0 {
                    // RM2000 (2000) message order: "<value><exp_received>".
                    self.log
                        .push(format!("{exp}{}", i18n::tr(&self.text.exp_received)));
                }
                if gold > 0 {
                    // RM2000 (2000): "<received_a> <value><gold><received_b>".
                    self.log.push(format!(
                        "{} {gold}{}{}",
                        i18n::tr(&self.text.gold_received_a),
                        i18n::tr(&self.text.gold),
                        i18n::tr(&self.text.gold_received_b),
                    ));
                }
            }
            BattleOutcome::Escape => {
                let line = i18n::tr(&self.text.escape_success);
                self.log.push(line);
            }
            BattleOutcome::Defeat => {
                let line = i18n::tr(&self.text.defeat);
                self.log.push(line);
            }
        }
    }
}
