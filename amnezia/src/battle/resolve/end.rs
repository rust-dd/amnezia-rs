//! Battle end checks, escape rolls and the original outcome message script.

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
        let rewards = self
            .enemies
            .iter()
            .filter(|e| !e.fled)
            .map(|e| (e.exp, e.gold))
            .collect::<Vec<_>>();
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

    /// Stage the outcome text and reward totals. The outcome system pays rewards
    /// once, appends item/level-up pages and waits for the shared message window.
    pub fn finish(&mut self, outcome: BattleOutcome) {
        if self.outcome == Some(outcome) {
            return;
        }
        self.outcome_log_start = self.log.len();
        let announced_escape = self.phase == Phase::Escape;
        self.outcome_message = Default::default();
        self.messages.console.clear();
        self.reward_items.clear();
        self.outcome = Some(outcome);
        self.phase = Phase::Outcome;
        match outcome {
            BattleOutcome::Victory => {
                let (exp, gold) = self.victory_rewards();
                self.reward_exp = exp;
                self.reward_gold = gold;
                for enemy in &self.enemies {
                    if enemy.hp <= 0
                        && !enemy.fled
                        && enemy.drop_id != 0
                        && self.items.iter().any(|item| item.id == enemy.drop_id)
                        && rng_next(&mut self.rng) % 100 < u64::from(enemy.drop_prob)
                    {
                        self.reward_items.push(enemy.drop_id);
                    }
                }
                self.outcome_line(i18n::tr(&self.text.victory), "\\|");
                if exp > 0 {
                    self.outcome_line(format!("{exp}{}", i18n::tr(&self.text.exp_received)), "\\.");
                }
                if gold > 0 {
                    self.outcome_line(
                        format!(
                            "{} {gold}{}{}",
                            i18n::tr(&self.text.gold_recieved_a),
                            i18n::tr(&self.text.gold),
                            i18n::tr(&self.text.gold_recieved_b),
                        ),
                        "\\.",
                    );
                }
            }
            BattleOutcome::Escape => {
                let line = i18n::tr(&self.text.escape_success);
                if !announced_escape {
                    self.log.push(line);
                }
            }
            BattleOutcome::Defeat => {
                let line = i18n::tr(&self.text.defeat);
                self.outcome_line(line, "");
            }
            BattleOutcome::Abort => {}
        }
    }

    fn outcome_line(&mut self, line: String, pause: &str) {
        self.outcome_message.reward(&line, pause);
        self.log.push(line);
    }
}
