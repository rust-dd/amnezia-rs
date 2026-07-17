//! Battle resolution: apply the agility-ordered turn queue to the live
//! [`Battle`], one action at a time — physical hits, skill casts, item heals,
//! defence, and enemy attacks — then check for the end, tally rewards, and roll a
//! flee. Extends [`Battle`] with a second `impl` block so the turn-flow half stays
//! in [`super::model`]; the damage numbers themselves come from [`super::logic`].

use super::BattleOutcome;
use super::logic;
use super::model::{Action, Battle, Command, Phase, Source, rng_next};
use amnezia_data::SkillDef;

/// Flat HP a medicine item restores in v1. `ItemDef` carries no heal magnitude,
/// so item potency is a documented constant rather than data-driven.
const ITEM_HEAL: i32 = 100;

/// The outcome of a party member's weapon strike: a clean miss, or a landed hit
/// carrying the damage dealt and whether it critical'd (for the log line).
enum Strike {
    Miss,
    Hit { dmg: i32, crit: bool },
}

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
                if logic::worst_restriction(&self.members[pi].states, &self.states) == 3 {
                    let Some(ti) = self.retarget_ally(pi, target) else {
                        return;
                    };
                    let base = logic::physical_damage(
                        self.members[pi].stats.attack,
                        self.members[ti].stats.defense,
                    );
                    let dmg = self.hit_member(ti, base);
                    format!(
                        "{} zavartan lesújt: {} -{}",
                        self.members[pi].name, self.members[ti].name, dmg
                    )
                } else {
                    let Some(ti) = self.retarget_enemy(target) else {
                        return;
                    };
                    let strike = self.strike_enemy(pi, ti);
                    let member = &self.members[pi].name;
                    let enemy = &self.enemies[ti].name;
                    match strike {
                        Strike::Miss => format!("{member} rácsap: {enemy} elkerülte"),
                        Strike::Hit { dmg, crit: true } => {
                            format!("{member} rácsap: Kritikus! {enemy} -{dmg}")
                        }
                        Strike::Hit { dmg, crit: false } => {
                            format!("{member} rácsap: {enemy} -{dmg}")
                        }
                    }
                }
            }
            (Source::Party(pi), Command::Skill { skill_id, target }) => {
                match self.cast_skill(pi, skill_id, target) {
                    Some(line) => line,
                    None => return,
                }
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
                if logic::worst_restriction(&self.enemies[ei].states, &self.states) == 3 {
                    let Some(ti) = self.retarget_other_enemy(ei, target) else {
                        return;
                    };
                    let base = logic::physical_damage(
                        self.enemies[ei].stats.attack,
                        self.enemies[ti].stats.defense,
                    );
                    let dmg = self.hit_enemy(ti, base);
                    format!(
                        "{} zavartan lesújt: {} -{}",
                        self.enemies[ei].name, self.enemies[ti].name, dmg
                    )
                } else {
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
            }
            (Source::Enemy(ei), Command::Skill { skill_id, target }) => {
                match self.enemy_cast(ei, skill_id, target) {
                    Some(line) => line,
                    None => return,
                }
            }
            (Source::Enemy(_), _) => return,
        };
        self.log.push(line);
    }

    /// Resolve a party member's weapon strike on enemy `ti`: a to-hit roll (bare
    /// hands default 90%), then on a hit the weapon's element against the foe's
    /// resistance ranks, a critical that triples, and finally ±10% variance.
    fn strike_enemy(&mut self, pi: usize, ti: usize) -> Strike {
        let base = logic::physical_damage(
            self.members[pi].stats.attack,
            self.enemies[ti].stats.defense,
        );
        let hit_rate = logic::effective_hit(self.members[pi].weapon_hit);
        if (rng_next(&mut self.rng) % 100) as u32 >= hit_rate {
            return Strike::Miss;
        }
        let element = self.members[pi].weapon_element.unwrap_or(0);
        let base = logic::elemental_damage(
            base,
            element,
            &self.enemies[ti].attribute_ranks,
            &self.attributes,
        );
        let crit = ((rng_next(&mut self.rng) % 100) as u32) < self.members[pi].weapon_crit;
        let base = if crit {
            logic::critical_damage(base)
        } else {
            base
        };
        let roll = (rng_next(&mut self.rng) % 21) as u32;
        let dmg = logic::with_variance(base, roll).max(0);
        self.enemies[ti].hp -= dmg;
        self.release_states_on_enemy(ti);
        Strike::Hit { dmg, crit }
    }

    fn hit_enemy(&mut self, ti: usize, base: i32) -> i32 {
        let roll = (rng_next(&mut self.rng) % 21) as u32;
        let dmg = logic::with_variance(base, roll).max(0);
        self.enemies[ti].hp -= dmg;
        self.release_states_on_enemy(ti);
        dmg
    }

    fn hit_member(&mut self, ti: usize, base: i32) -> i32 {
        let roll = (rng_next(&mut self.rng) % 21) as u32;
        let mut dmg = logic::with_variance(base, roll).max(0);
        if self.members[ti].defending {
            dmg = logic::defended(dmg);
        }
        self.members[ti].hp -= dmg;
        self.release_states_on_member(ti);
        dmg
    }

    /// Resolve member `pi`'s cast of skill `skill_id` at `target`: deduct SP, then
    /// apply the scoped effect — an elemental attack that may inflict states
    /// (enemy scope 0/1), or a heal that may cure them (ally scope 2/3/4). Returns
    /// the joined log line(s), or `None` for an unknown skill id (no effect).
    fn cast_skill(&mut self, pi: usize, skill_id: u32, target: usize) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id).cloned()?;
        self.members[pi].sp = (self.members[pi].sp - skill.sp_cost as i32).max(0);
        let mut lines: Vec<String> = Vec::new();
        match skill.scope {
            1 => {
                for ti in self.living_enemies() {
                    lines.extend(self.skill_hit_enemy(pi, ti, &skill));
                }
            }
            2 => lines.extend(self.skill_heal_ally(pi, pi, &skill)),
            3 => {
                if self.members.get(target).is_some_and(|m| m.alive()) {
                    lines.extend(self.skill_heal_ally(pi, target, &skill));
                }
            }
            4 => {
                for ti in 0..self.members.len() {
                    if self.members[ti].alive() {
                        lines.extend(self.skill_heal_ally(pi, ti, &skill));
                    }
                }
            }
            _ => {
                if let Some(ti) = self.retarget_enemy(target) {
                    lines.extend(self.skill_hit_enemy(pi, ti, &skill));
                }
            }
        }
        let caster = self.members[pi].name.clone();
        Some(if lines.is_empty() {
            format!("{caster} varázsol")
        } else {
            lines.join("\n")
        })
    }

    /// Land `skill` from caster `pi` on enemy `ti`: elemental damage against the
    /// foe's resistance ranks (SP drains find no pool on a foe), an optional
    /// life-absorb for the caster, and a status-infliction roll per affected
    /// state, weighted by the foe's affliction rank.
    fn skill_hit_enemy(&mut self, pi: usize, ti: usize, skill: &SkillDef) -> Vec<String> {
        let caster = self.members[pi].name.clone();
        let target = self.enemies[ti].name.clone();
        let base = logic::skill_damage(
            skill.power,
            self.members[pi].stats.spirit,
            self.enemies[ti].stats.spirit,
        );
        let element = skill.attributes.first().copied().unwrap_or(0);
        let base = logic::elemental_damage(
            base,
            element,
            &self.enemies[ti].attribute_ranks,
            &self.attributes,
        );
        // Foes carry no SP pool, so an SP-draining skill finds nothing to take.
        let dealt = if skill.affect_sp {
            0
        } else {
            self.hit_enemy(ti, base)
        };
        if skill.absorb && dealt > 0 {
            let f = &mut self.members[pi];
            f.hp = (f.hp + dealt).min(f.max_hp);
        }
        let mut lines = vec![format!("{caster} varázsol: {target} -{dealt}")];
        for &sid in &skill.affected_states {
            let rank = sid
                .checked_sub(1)
                .and_then(|i| self.enemies[ti].state_ranks.get(i as usize).copied())
                .unwrap_or(2);
            if ((rng_next(&mut self.rng) % 100) as u32) < logic::state_infliction_chance(rank) {
                logic::inflict(&mut self.enemies[ti].states, sid);
                if let Some(state) = self.states.iter().find(|s| s.id == sid) {
                    lines.push(format!("{target} státusz: {}", state.name));
                }
            }
        }
        lines
    }

    /// Heal ally `ti` for caster `pi`'s `skill`: restore SP or HP (clamped to the
    /// maximum), then cure each of the skill's affected states from that ally.
    fn skill_heal_ally(&mut self, pi: usize, ti: usize, skill: &SkillDef) -> Vec<String> {
        let caster = self.members[pi].name.clone();
        let target = self.members[ti].name.clone();
        let base = logic::skill_damage(
            skill.power,
            self.members[pi].stats.spirit,
            self.members[ti].stats.spirit,
        );
        let roll = (rng_next(&mut self.rng) % 21) as u32;
        let amt = logic::with_variance(base, roll).max(0);
        let f = &mut self.members[ti];
        if skill.affect_sp {
            f.sp = (f.sp + amt).min(f.max_sp);
        } else {
            f.hp = (f.hp + amt).min(f.max_hp);
        }
        let mut lines = vec![format!("{caster} varázsol: {target} +{amt}")];
        for &sid in &skill.affected_states {
            if logic::has_state(&self.members[ti].states, sid) {
                logic::cure(&mut self.members[ti].states, sid);
                if let Some(state) = self.states.iter().find(|s| s.id == sid) {
                    lines.push(format!("{target} gyógyul: {}", state.name));
                }
            }
        }
        lines
    }

    /// Choose living enemy `i`'s action for the coming round. A status restriction
    /// overrides the AI: can't-act does nothing, berserk forces a plain attack on a
    /// party member, confusion an attack on a random other foe (friendly fire).
    /// Otherwise a random living party target, this foe's and the party's HP
    /// percentages, and the round number feed [`logic::choose_enemy_action`], whose
    /// result [`logic::enemy_command`] maps to a [`Command`] (a basic attack when
    /// nothing is eligible). `None` when the enemy is down or no member is left to
    /// target.
    pub(super) fn enemy_action(&mut self, i: usize, alive: &[bool]) -> Option<Action> {
        if !self.enemies[i].alive() {
            return None;
        }
        let agility = self.enemies[i].stats.agility;
        let restriction = logic::worst_restriction(&self.enemies[i].states, &self.states);
        let kind = match restriction {
            1 => Command::Nothing,
            3 => {
                // Confusion turns the blow on a random other living foe (resolved
                // as friendly fire in `apply`); none left -> nothing.
                let others: Vec<bool> = self
                    .enemies
                    .iter()
                    .enumerate()
                    .map(|(j, e)| j != i && e.alive())
                    .collect();
                match logic::select_target(&others, rng_next(&mut self.rng) as usize) {
                    Some(target) => Command::Attack { target },
                    None => Command::Nothing,
                }
            }
            2 => {
                // Berserk forces a plain attack on a random living party member.
                let target = logic::select_target(alive, rng_next(&mut self.rng) as usize)?;
                Command::Attack { target }
            }
            _ => {
                let target = logic::select_target(alive, rng_next(&mut self.rng) as usize)?;
                let party_hp: i32 = self.members.iter().map(|f| f.hp.max(0)).sum();
                let party_max: i32 = self.members.iter().map(|f| f.max_hp).sum();
                let enemy_hp_pct = logic::hp_percent(self.enemies[i].hp, self.enemies[i].max_hp);
                let party_hp_pct = logic::hp_percent(party_hp, party_max);
                let chosen = logic::choose_enemy_action(
                    &self.enemies[i].actions,
                    enemy_hp_pct,
                    party_hp_pct,
                    logic::AI_PARTY_LEVEL,
                    self.round,
                    rng_next(&mut self.rng),
                );
                logic::enemy_command(chosen.as_ref(), target)
            }
        };
        Some(Action {
            source: Source::Enemy(i),
            kind,
            agility,
        })
    }

    /// Resolve enemy `ei`'s cast of `skill_id` at member `target`: an ally-scope
    /// skill (scope 2/3/4) heals the caster itself (a foe keeps no ally list),
    /// clamped to its max HP; any other scope damages the member (halved while it
    /// defends, like a physical hit). `None` for an unknown skill id.
    fn enemy_cast(&mut self, ei: usize, skill_id: u32, target: usize) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id).cloned()?;
        let spirit = self.enemies[ei].stats.spirit;
        let name = self.enemies[ei].name.clone();
        if matches!(skill.scope, 2..=4) {
            let base = logic::skill_damage(skill.power, spirit, 0);
            let roll = (rng_next(&mut self.rng) % 21) as u32;
            let amt = logic::with_variance(base, roll).max(0);
            let e = &mut self.enemies[ei];
            e.hp = (e.hp + amt).min(e.max_hp);
            return Some(format!("{name} varázsol: {name} +{amt}"));
        }
        let ti = self.retarget_member(target)?;
        let base = logic::skill_damage(skill.power, spirit, self.members[ti].stats.spirit);
        let dmg = self.hit_member(ti, base);
        Some(format!("{name} varázsol: {} -{dmg}", self.members[ti].name))
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

    /// Keep confused member `pi`'s stored ally `target` if it still lives, else pick
    /// another living ally. `None` when `pi` has no living ally to turn on.
    fn retarget_ally(&mut self, pi: usize, target: usize) -> Option<usize> {
        if target != pi && self.members.get(target).is_some_and(|m| m.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self
            .members
            .iter()
            .enumerate()
            .map(|(j, m)| j != pi && m.alive())
            .collect();
        logic::select_target(&alive, rng_next(&mut self.rng) as usize)
    }

    /// Keep confused foe `ei`'s stored fellow `target` if it still lives, else pick
    /// another living foe. `None` when `ei` has no living fellow to turn on.
    fn retarget_other_enemy(&mut self, ei: usize, target: usize) -> Option<usize> {
        if target != ei && self.enemies.get(target).is_some_and(|e| e.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self
            .enemies
            .iter()
            .enumerate()
            .map(|(j, e)| j != ei && e.alive())
            .collect();
        logic::select_target(&alive, rng_next(&mut self.rng) as usize)
    }

    /// Advance the command phase past every member the game must act for: a
    /// can't-act (restriction 1) member is auto-ordered [`Command::Nothing`]; a
    /// berserk (2) member is forced to strike a random living enemy; a confused (3)
    /// member a random living ally. Stops on the first member who may freely choose
    /// (setting [`Battle::turn`]), or enters resolution once every remaining chooser
    /// has been auto-ordered — so the command UI never halts on a restricted member.
    pub(super) fn skip_restricted_choosers(&mut self) {
        while let Some(i) = self.next_chooser() {
            let restriction = logic::worst_restriction(&self.members[i].states, &self.states);
            if restriction == 0 {
                self.turn = i;
                return;
            }
            let forced = self.forced_party_command(i, restriction);
            self.members[i].command = Some(forced);
        }
        self.begin_resolve();
    }

    /// The attack a restricted member `i` is forced into: a random living enemy
    /// while berserk (2), a random living ally while confused (3), or nothing at all
    /// (restriction 1, or no legal target). The chosen side is re-checked and the
    /// blow re-aimed at resolution by [`Battle::apply`].
    fn forced_party_command(&mut self, i: usize, restriction: u32) -> Command {
        let alive: Vec<bool> = match restriction {
            2 => self.enemies.iter().map(|e| e.alive()).collect(),
            3 => self
                .members
                .iter()
                .enumerate()
                .map(|(j, m)| j != i && m.alive())
                .collect(),
            _ => return Command::Nothing,
        };
        match logic::select_target(&alive, rng_next(&mut self.rng) as usize) {
            Some(target) => Command::Attack { target },
            None => Command::Nothing,
        }
    }

    /// Wear off timed states at the top of a round: for every combatant advance
    /// each active state's held-turn count and roll its auto-release once past its
    /// hold turns (the death state is exempt), logging whatever lifts.
    ///
    // TODO: per-turn HP effects (e.g. poison drain) are not applied — `StateDef`
    // carries no HP-change fields in the converted data yet, so there is nothing
    // to drain from here. Revisit once those fields are parsed.
    pub(super) fn run_recovery(&mut self) {
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
    fn release_states_on_member(&mut self, ti: usize) {
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
    fn release_states_on_enemy(&mut self, ti: usize) {
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

    fn fire_attr() -> amnezia_data::AttributeDef {
        amnezia_data::AttributeDef {
            id: 5,
            name: "Tűz".into(),
            attribute_type: 0,
            a_rate: 200, // rank A: weak, double damage
            b_rate: 150,
            c_rate: 100,
            d_rate: 50,
            e_rate: 0, // rank E: immune, no damage
        }
    }

    /// A single-enemy (scope 0) damage skill carrying `attributes` (elements) and
    /// `states` (statuses it may inflict).
    fn damage_skill(id: u32, power: u32, attributes: Vec<u32>, states: Vec<u32>) -> SkillDef {
        SkillDef {
            id,
            name: "S".into(),
            description: String::new(),
            sp_cost: 3,
            power,
            hit: 0,
            skill_type: 0,
            scope: 0,
            physical_rate: 0,
            magical_rate: 3,
            affect_hp: true,
            affect_sp: false,
            absorb: false,
            attributes,
            affected_states: states,
        }
    }

    /// A single-ally (scope 3) HP heal.
    fn heal_skill(id: u32, power: u32) -> SkillDef {
        let mut s = damage_skill(id, power, vec![], vec![]);
        s.scope = 3;
        s
    }

    fn poison_state(id: u32) -> amnezia_data::StateDef {
        amnezia_data::StateDef {
            id,
            name: "Méreg".into(),
            restriction: 0,
            priority: 50,
            hold_turn: 0,
            auto_release_prob: 0,
            release_by_damage: 0,
        }
    }

    #[test]
    fn a_party_attack_wounds_its_target_and_logs() {
        let mut battle = build_1v2();
        // Force a guaranteed hit so the wound assertion doesn't ride on the new
        // 90% bare-hands to-hit roll (which would miss 10% of seeds).
        battle.members[0].weapon_hit = 100;
        battle.commit(Command::Attack { target: 0 });
        let before = battle.enemies[0].hp;
        while battle.resolve_next() {}
        assert!(battle.enemies[0].hp < before);
        assert!(battle.log.iter().any(|l| l.contains("rácsap")));
    }

    #[test]
    fn a_forced_critical_triples_the_blow() {
        let mut battle = build_1v2();
        battle.members[0].weapon_hit = 100; // never miss
        battle.members[0].weapon_crit = 100; // always crit
        let base = logic::physical_damage(
            battle.members[0].stats.attack,
            battle.enemies[0].stats.defense,
        );
        let Strike::Hit { dmg, crit } = battle.strike_enemy(0, 0) else {
            panic!("a forced-hit strike missed");
        };
        assert!(crit);
        // ±10% variance around the tripled base leaves it in this band, and it
        // clearly beats a plain blow either way.
        let tripled = base * 3;
        assert!(dmg >= tripled - tripled / 10 && dmg <= tripled + tripled / 10);
        assert!(dmg > base);
    }

    #[test]
    fn a_missed_strike_deals_no_damage() {
        let mut battle = build_1v2();
        battle.members[0].weapon_hit = 90; // bare-hands default: misses 10% of rolls
        // Wind the rng to a state whose next to-hit roll falls in the miss band.
        loop {
            let mut probe = battle.rng;
            if rng_next(&mut probe) % 100 >= 90 {
                break;
            }
            rng_next(&mut battle.rng);
        }
        let before = battle.enemies[0].hp;
        let strike = battle.strike_enemy(0, 0);
        assert!(matches!(strike, Strike::Miss));
        assert_eq!(battle.enemies[0].hp, before); // a miss deals 0
    }

    #[test]
    fn an_elemental_strike_amplifies_against_a_weak_foe() {
        let mut battle = build_1v2();
        battle.members[0].weapon_hit = 100; // never miss
        battle.members[0].weapon_crit = 0; // never crit — isolate the element
        battle.members[0].weapon_element = Some(5); // fire
        battle.attributes = vec![fire_attr()];
        battle.enemies[0].attribute_ranks = vec![2, 2, 2, 2, 0]; // attr 5 -> rank A (weak)
        let base = logic::physical_damage(
            battle.members[0].stats.attack,
            battle.enemies[0].stats.defense,
        );
        let Strike::Hit { dmg, .. } = battle.strike_enemy(0, 0) else {
            panic!("a forced-hit strike missed");
        };
        assert!(dmg > base); // a weak (A) rank amplifies past the plain hit
    }

    #[test]
    fn an_elemental_strike_is_nullified_by_an_immune_foe() {
        let mut battle = build_1v2();
        battle.members[0].weapon_hit = 100; // never miss
        battle.members[0].weapon_element = Some(5); // fire
        battle.attributes = vec![fire_attr()];
        battle.enemies[0].attribute_ranks = vec![2, 2, 2, 2, 4]; // attr 5 -> rank E (0%)
        let before = battle.enemies[0].hp;
        let Strike::Hit { dmg, .. } = battle.strike_enemy(0, 0) else {
            panic!("a forced-hit strike missed");
        };
        assert_eq!(dmg, 0); // immune (E, 0%) nullifies the blow
        assert_eq!(battle.enemies[0].hp, before);
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

    #[test]
    fn a_fire_skill_amplifies_against_a_weak_foe() {
        let mut battle = build_1v2();
        battle.attributes = vec![fire_attr()]; // element id 5
        battle.enemies[0].attribute_ranks = vec![2, 2, 2, 2, 0]; // attr 5 -> rank A (weak)
        battle.skills = vec![damage_skill(1, 50, vec![5], vec![])];
        let plain = logic::skill_damage(
            50,
            battle.members[0].stats.spirit,
            battle.enemies[0].stats.spirit,
        );
        let before = battle.enemies[0].hp;
        battle.commit(Command::Skill {
            skill_id: 1,
            target: 0,
        });
        while battle.resolve_next() {}
        let dealt = before - battle.enemies[0].hp;
        assert!(
            dealt > plain,
            "fire against a weak (A) foe should beat the plain base ({dealt} <= {plain})"
        );
    }

    #[test]
    fn an_ally_heal_raises_a_wounded_ally_clamped_to_max() {
        let mut battle = build_1v2();
        battle.skills = vec![heal_skill(2, 40)]; // scope 3, power 40
        let max = battle.members[0].max_hp;
        battle.members[0].hp = max - 3; // wounded, within one heal of full
        battle.cast_skill(0, 2, 0);
        assert_eq!(battle.members[0].hp, max); // healed past the deficit, clamped to max
    }

    #[test]
    fn a_damage_skill_inflicts_its_state_on_a_forced_hit_roll() {
        let mut battle = build_1v2();
        battle.states = vec![poison_state(3)];
        battle.enemies[0].hp = 200; // survive the blow so the status lands on a live foe
        battle.enemies[0].state_ranks = vec![2, 2, 0]; // state 3 -> rank A (100% infliction)
        battle.skills = vec![damage_skill(1, 20, vec![], vec![3])];
        battle.cast_skill(0, 1, 0);
        assert!(logic::has_state(&battle.enemies[0].states, 3));
    }

    #[test]
    fn an_enemy_skill_cast_wounds_the_targeted_member() {
        let mut battle = build_1v2();
        battle.skills = vec![damage_skill(1, 30, vec![], vec![])]; // scope 0 -> hits a member
        let before = battle.members[0].hp;
        let line = battle.enemy_cast(0, 1, 0).unwrap();
        assert!(battle.members[0].hp < before);
        assert!(line.contains("varázsol"));
    }

    #[test]
    fn an_enemy_ally_scope_skill_heals_the_caster_clamped_to_max() {
        let mut battle = build_1v2();
        battle.skills = vec![heal_skill(2, 40)]; // scope 3 -> caster heals itself
        battle.enemies[0].hp = battle.enemies[0].max_hp - 5; // wounded, within one heal of full
        battle.enemy_cast(0, 2, 0);
        assert_eq!(battle.enemies[0].hp, battle.enemies[0].max_hp);
    }

    fn damage_release_state(id: u32) -> amnezia_data::StateDef {
        amnezia_data::StateDef {
            id,
            name: "Bódulat".into(),
            restriction: 0,
            priority: 0,
            hold_turn: 0,
            auto_release_prob: 0,
            release_by_damage: 100,
        }
    }

    fn confusion_state(id: u32) -> amnezia_data::StateDef {
        amnezia_data::StateDef {
            id,
            name: "Zavar".into(),
            restriction: 3,
            priority: 0,
            hold_turn: 0,
            auto_release_prob: 0,
            release_by_damage: 0,
        }
    }

    #[test]
    fn being_hit_wears_off_a_damage_release_state_but_never_death() {
        let mut battle = build_1v2();
        // id 1 is the death state (exempt); id 2 shakes off on any hit.
        battle.states = vec![poison_state(1), damage_release_state(2)];
        battle.members[0].states = vec![(1, 0), (2, 0)];
        battle.hit_member(0, 8);
        assert!(logic::has_state(&battle.members[0].states, 1)); // KO exempt
        assert!(!logic::has_state(&battle.members[0].states, 2)); // lifted by the blow
    }

    #[test]
    fn a_confused_member_turns_on_a_living_ally() {
        use super::super::model::testkit;
        use crate::progression::Progression;
        use crate::vitals::Vitals;
        let ron = testkit::actor(1, 5, 80, 40);
        let tiff = testkit::actor(2, 5, 80, 40);
        let actors = vec![&ron, &tiff];
        let monsters = vec![testkit::monster(1, 30, 10, 30)];
        let troop = testkit::troop(&[(1, 100, 100)]);
        let mut battle = Battle::build(
            &troop,
            &monsters,
            &actors,
            &[],
            &[],
            &[],
            &[],
            &Vitals::default(),
            &Progression::default(),
            "Cave1".into(),
            3,
        );
        battle.states = vec![confusion_state(9)];
        battle.members[0].states = vec![(9, 0)]; // member 0 is confused
        let ally_hp = battle.members[1].hp;
        // The command flow auto-orders the confused member to strike an ally.
        battle.skip_restricted_choosers();
        assert!(matches!(
            battle.members[0].command,
            Some(Command::Attack { .. })
        ));
        battle.commit(Command::Defend); // member 1 (free) finishes the round
        while battle.resolve_next() {}
        assert!(battle.members[1].hp < ally_hp);
    }
}
