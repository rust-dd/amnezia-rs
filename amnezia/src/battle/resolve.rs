//! Battle resolution: apply the agility-ordered turn queue to the live
//! [`Battle`], one action at a time — physical hits, skill casts, item heals,
//! defence, and enemy attacks — then check for the end, tally rewards, and roll a
//! flee. Extends [`Battle`] with a second `impl` block so the turn-flow half stays
//! in [`super::model`]; the damage numbers themselves come from [`super::logic`].

use super::BattleOutcome;
use super::logic;
use super::model::{Action, Battle, Command, Phase, Source, rng_next};

/// Flat HP a medicine item restores in v1. `ItemDef` carries no heal magnitude,
/// so item potency is a documented constant rather than data-driven.
const ITEM_HEAL: i32 = 100;

impl Battle {
    /// Apply the next queued action (skipping a fainted actor, retargeting a dead
    /// target) and append its log line. Returns `false` once the queue is spent.
    pub fn resolve_next(&mut self) -> bool {
        let Some(&action) = self.queue.get(self.queue_at) else {
            return false;
        };
        self.queue_at += 1;
        if self.source_alive(action.source) {
            self.apply(action);
        }
        true
    }

    fn source_alive(&self, source: Source) -> bool {
        match source {
            Source::Party(i) => self.members.get(i).is_some_and(|f| f.alive()),
            Source::Enemy(i) => self.enemies.get(i).is_some_and(|e| e.alive()),
        }
    }

    fn apply(&mut self, action: Action) {
        let line = match (action.source, action.kind) {
            (Source::Party(pi), Command::Attack { target }) => {
                let attack = self.members[pi].stats.attack;
                let Some(ti) = self.retarget_enemy(target) else {
                    return;
                };
                let base = logic::physical_damage(attack, self.enemies[ti].stats.defense);
                let dmg = self.hit_enemy(ti, base);
                format!(
                    "{} rácsap: {} -{}",
                    self.members[pi].name, self.enemies[ti].name, dmg
                )
            }
            (
                Source::Party(pi),
                Command::Skill {
                    power,
                    cost,
                    target,
                },
            ) => {
                self.members[pi].sp = (self.members[pi].sp - cost as i32).max(0);
                let spirit = self.members[pi].stats.spirit;
                let Some(ti) = self.retarget_enemy(target) else {
                    return;
                };
                let base = logic::skill_damage(power, spirit, self.enemies[ti].stats.spirit);
                let dmg = self.hit_enemy(ti, base);
                format!(
                    "{} varázsol: {} -{}",
                    self.members[pi].name, self.enemies[ti].name, dmg
                )
            }
            (Source::Party(pi), Command::Item) => {
                let f = &mut self.members[pi];
                f.hp = (f.hp + ITEM_HEAL).min(f.max_hp);
                format!("{} gyógyul +{}", f.name, ITEM_HEAL)
            }
            (Source::Party(pi), Command::Defend) => {
                self.members[pi].defending = true;
                format!("{} védekezik", self.members[pi].name)
            }
            (Source::Party(pi), Command::Nothing) => {
                format!("{} tétovázik", self.members[pi].name)
            }
            (Source::Enemy(ei), Command::Attack { target }) => {
                let attack = self.enemies[ei].stats.attack;
                let Some(ti) = self.retarget_member(target) else {
                    return;
                };
                let base = logic::physical_damage(attack, self.members[ti].stats.defense);
                let dmg = self.hit_member(ti, base);
                format!(
                    "{} támad: {} -{}",
                    self.enemies[ei].name, self.members[ti].name, dmg
                )
            }
            (Source::Enemy(_), _) => return,
        };
        self.log.push(line);
    }

    fn hit_enemy(&mut self, ti: usize, base: i32) -> i32 {
        let roll = (rng_next(&mut self.rng) % 21) as u32;
        let dmg = logic::with_variance(base, roll).max(0);
        self.enemies[ti].hp -= dmg;
        dmg
    }

    fn hit_member(&mut self, ti: usize, base: i32) -> i32 {
        let roll = (rng_next(&mut self.rng) % 21) as u32;
        let mut dmg = logic::with_variance(base, roll).max(0);
        if self.members[ti].defending {
            dmg = logic::defended(dmg);
        }
        self.members[ti].hp -= dmg;
        dmg
    }

    /// Keep `target` if that enemy still lives, else pick another living enemy.
    fn retarget_enemy(&mut self, target: usize) -> Option<usize> {
        if self.enemies.get(target).is_some_and(|e| e.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self.enemies.iter().map(|e| e.alive()).collect();
        let roll = rng_next(&mut self.rng) as usize;
        logic::select_target(&alive, roll)
    }

    fn retarget_member(&mut self, target: usize) -> Option<usize> {
        if self.members.get(target).is_some_and(|m| m.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self.members.iter().map(|m| m.alive()).collect();
        let roll = rng_next(&mut self.rng) as usize;
        logic::select_target(&alive, roll)
    }

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

    /// Total `(exp, gold)` for defeating the whole troop.
    pub fn victory_rewards(&self) -> (u32, u32) {
        let rewards: Vec<(u32, u32)> = self.enemies.iter().map(|e| (e.exp, e.gold)).collect();
        logic::total_rewards(&rewards)
    }

    /// Roll a party escape against the enemies' average agility.
    pub fn attempt_flee(&mut self) -> bool {
        let party = self
            .members
            .iter()
            .filter(|m| m.alive())
            .map(|m| m.stats.agility)
            .max();
        let living: Vec<u32> = self
            .enemies
            .iter()
            .filter(|e| e.alive())
            .map(|e| e.stats.agility)
            .collect();
        let enemy = if living.is_empty() {
            0
        } else {
            living.iter().sum::<u32>() / living.len() as u32
        };
        let chance = logic::flee_chance(party.unwrap_or(0), enemy);
        let roll = (rng_next(&mut self.rng) % 100) as u32;
        logic::flee_succeeds(chance, roll)
    }

    /// Conclude the fight with `outcome`: record the reward on victory and push the
    /// closing log line, then wait in the outcome phase for the player.
    pub fn finish(&mut self, outcome: BattleOutcome) {
        self.outcome = Some(outcome);
        self.phase = Phase::Outcome;
        let line = match outcome {
            BattleOutcome::Victory => {
                let (exp, gold) = self.victory_rewards();
                self.reward_exp = exp;
                self.reward_gold = gold;
                format!("Győzelem! +{exp} EXP, +{gold} arany")
            }
            BattleOutcome::Escape => "Sikeres menekülés!".to_string(),
            BattleOutcome::Defeat => "Vereség...".to_string(),
        };
        self.log.push(line);
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::testkit::build_1v2;
    use super::*;

    #[test]
    fn a_party_attack_wounds_its_target_and_logs() {
        let mut battle = build_1v2();
        battle.commit(Command::Attack { target: 0 });
        let before = battle.enemies[0].hp;
        while battle.resolve_next() {}
        assert!(battle.enemies[0].hp < before);
        assert!(battle.log.iter().any(|l| l.contains("rácsap")));
    }

    #[test]
    fn killing_every_enemy_yields_victory_and_summed_reward() {
        let mut battle = build_1v2();
        for e in &mut battle.enemies {
            e.hp = 0;
        }
        assert!(matches!(battle.end_state(), Some(BattleOutcome::Victory)));
        assert_eq!(battle.victory_rewards(), (20, 60)); // 10+10 exp, 30+30 gold
    }

    #[test]
    fn wiping_the_party_yields_defeat() {
        let mut battle = build_1v2();
        battle.members[0].hp = 0;
        assert!(matches!(battle.end_state(), Some(BattleOutcome::Defeat)));
    }

    #[test]
    fn retargeting_skips_a_dead_enemy() {
        let mut battle = build_1v2();
        battle.enemies[0].hp = 0; // first target dead
        assert_eq!(battle.retarget_enemy(0), Some(1)); // falls through to the living one
    }

    #[test]
    fn defend_halves_the_hit_a_member_takes() {
        let mut battle = build_1v2();
        battle.members[0].defending = true;
        let full = battle.members[0].hp;
        battle.hit_member(0, 8); // base 8, halved by defence to ~4
        assert!(full - battle.members[0].hp <= 4);
    }

    #[test]
    fn finish_victory_records_reward_and_log() {
        let mut battle = build_1v2();
        battle.finish(BattleOutcome::Victory);
        assert!(battle.phase == Phase::Outcome);
        assert_eq!(battle.reward_gold, 60);
        assert!(battle.log_tail().contains("Győzelem"));
    }
}
