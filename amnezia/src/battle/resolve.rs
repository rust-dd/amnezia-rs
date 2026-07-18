//! Battle resolution: apply the agility-ordered turn queue to the live
//! [`Battle`], one action at a time — physical hits, skill casts, item heals,
//! defence, and enemy attacks — then check for the end, tally rewards, and roll a
//! flee. Extends [`Battle`] with a second `impl` block so the turn-flow half stays
//! in [`super::model`]; the damage numbers themselves come from [`super::logic`].

use super::BattleOutcome;
use super::logic;
use super::model::{Action, Battle, Command, Phase, Source, rng_next};
use amnezia_data::SkillDef;

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
            // RM2000 applies an HP-changing state (poison drain, regen) at the
            // start of the battler's turn, before it acts; a drain that fells the
            // battler cancels its action through the HP-based death path.
            self.tick_state_hp(action.source);
            if self.source_alive(action.source) {
                self.apply(action);
            }
        }
        true
    }

    fn source_alive(&self, source: Source) -> bool {
        match source {
            Source::Party(i) => self.members.get(i).is_some_and(|f| f.alive()),
            Source::Enemy(i) => self.enemies.get(i).is_some_and(|e| e.alive()),
        }
    }

    /// Apply every active HP-changing state to `source` at the start of its turn
    /// (RM2000 timing, before it acts): a type-0 state drains and a type-1 state
    /// regenerates [`logic::state_hp_delta`]'s per-turn amount, a type-2 (and any
    /// unconfigured) state does nothing. A drain floors HP at 0 — from there the
    /// battler is defeated by the existing HP-based death path — and a regen caps
    /// at max HP. Each change is logged in the state-line style. Map/field HP
    /// change (`hp_change_map_*`) is deliberately skipped: it needs persistent
    /// overworld states, which battle state does not carry.
    fn tick_state_hp(&mut self, source: Source) {
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
                    let dmg = self.hit_member(ti, base, 4);
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
            (Source::Party(pi), Command::Item { item_id, target }) => {
                self.apply_item(pi, item_id, target)
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
                    let dmg = self.hit_enemy(ti, base, 4);
                    format!(
                        "{} zavartan lesújt: {} -{}",
                        self.enemies[ei].name, self.enemies[ti].name, dmg
                    )
                } else {
                    let Some(ti) = self.retarget_member(target) else {
                        return;
                    };
                    let enemy = self.enemies[ei].name.clone();
                    let member = self.members[ti].name.clone();
                    match self.enemy_strike_member(ei, ti) {
                        Some(dmg) => format!("{enemy} támad: {member} -{dmg}"),
                        None => format!("{enemy} támad: {member} elkerülte"),
                    }
                }
            }
            (Source::Enemy(ei), Command::Skill { skill_id, target }) => {
                match self.enemy_cast(ei, skill_id, target) {
                    Some(line) => line,
                    None => return,
                }
            }
            (Source::Enemy(ei), Command::DoubleAttack { target }) => {
                let Some(ti) = self.retarget_member(target) else {
                    return;
                };
                let d1 = self.enemy_strike_member(ei, ti);
                let d2 = self.enemy_strike_member(ei, ti);
                let enemy = self.enemies[ei].name.clone();
                let member = self.members[ti].name.clone();
                let show =
                    |d: Option<i32>| d.map_or_else(|| "elkerülte".to_string(), |v| format!("-{v}"));
                format!("{enemy} kétszer támad: {member} {}, {}", show(d1), show(d2))
            }
            (Source::Enemy(ei), Command::Defend) => {
                self.enemies[ei].defending = true;
                format!("{} védekezik", self.enemies[ei].name)
            }
            (Source::Enemy(ei), Command::SelfDestruct) => {
                let damage = self.enemies[ei].stats.attack as i32;
                let name = self.enemies[ei].name.clone();
                for ti in self.living_members() {
                    self.hit_member(ti, damage, 4);
                }
                self.enemies[ei].hp = 0;
                format!("{name} felrobban!")
            }
            (Source::Enemy(ei), Command::Escape) => {
                self.enemies[ei].fled = true;
                format!("{} elmenekül", self.enemies[ei].name)
            }
            (Source::Enemy(ei), Command::Charge) => {
                self.enemies[ei].charging = true;
                format!("{} erőt gyűjt", self.enemies[ei].name)
            }
            (Source::Enemy(ei), Command::Nothing) => {
                format!("{} tétovázik", self.enemies[ei].name)
            }
            _ => return,
        };
        self.log.push(line);
    }

    /// Resolve a party member's weapon strike on enemy `ti`, in RM2000 order: an
    /// agility-adjusted to-hit roll (bare hands default 90%), then on a hit the
    /// weapon's element against the foe's resistance ranks, a critical that
    /// triples, the `var=4` variance, and finally the defending-foe halving.
    fn strike_enemy(&mut self, pi: usize, ti: usize) -> Strike {
        let base = logic::physical_damage(
            self.members[pi].stats.attack,
            self.enemies[ti].stats.defense,
        );
        let hit = logic::to_hit(
            logic::effective_hit(self.members[pi].weapon_hit),
            self.members[pi].stats.agility,
            self.enemies[ti].stats.agility,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
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
        let roll = rng_next(&mut self.rng);
        let mut dmg = logic::variance_adjust(base, 4, roll).max(0);
        // A defending foe halves the final result (min 1 on a landed hit),
        // matching the member Defend and EasyRPG `AdjustDamageForDefend`, applied
        // after element/crit/variance.
        if self.enemies[ti].defending && dmg > 0 {
            dmg = logic::defended(dmg).max(1);
        }
        self.enemies[ti].hp -= dmg;
        self.release_states_on_enemy(ti);
        Strike::Hit { dmg, crit }
    }

    /// Apply `base` damage to enemy `ti` with `var` variance (4 for a physical
    /// blow, the skill's variance for a cast), one draw per hit, then the
    /// defending-foe halving (min 1). Returns the damage dealt.
    fn hit_enemy(&mut self, ti: usize, base: i32, var: i32) -> i32 {
        let roll = rng_next(&mut self.rng);
        let mut dmg = logic::variance_adjust(base, var, roll).max(0);
        if self.enemies[ti].defending && dmg > 0 {
            dmg = logic::defended(dmg).max(1);
        }
        self.enemies[ti].hp -= dmg;
        self.release_states_on_enemy(ti);
        dmg
    }

    /// Apply `base` damage to member `ti` with `var` variance (4 for a physical
    /// blow, the skill's variance for a cast), one draw per hit, then the member's
    /// own defend halving. Returns the damage dealt.
    fn hit_member(&mut self, ti: usize, base: i32, var: i32) -> i32 {
        let roll = rng_next(&mut self.rng);
        let mut dmg = logic::variance_adjust(base, var, roll).max(0);
        if self.members[ti].defending {
            dmg = logic::defended(dmg);
        }
        self.members[ti].hp -= dmg;
        self.release_states_on_member(ti);
        dmg
    }

    /// One enemy `ei` physical strike on member `ti`: an agility-adjusted to-hit
    /// roll off the RM2000 90% bare-hands base that returns `None` on a miss, else
    /// the dealt damage via `hit_member` (its variance and the member's own defend
    /// halving). A pending charge-up doubles the blow; it is spent on the swing
    /// whether or not the blow lands, so the foe's next strike is normal again.
    fn enemy_strike_member(&mut self, ei: usize, ti: usize) -> Option<i32> {
        let charged = self.enemies[ei].charging;
        self.enemies[ei].charging = false;
        let hit = logic::to_hit(
            logic::effective_hit(0),
            self.enemies[ei].stats.agility,
            self.members[ti].stats.agility,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
            return None;
        }
        let mut base = logic::physical_damage(
            self.enemies[ei].stats.attack,
            self.members[ti].stats.defense,
        );
        if charged {
            base *= 2;
        }
        Some(self.hit_member(ti, base, 4))
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
        let base = logic::skill_effect(
            skill,
            &self.members[pi].stats,
            &self.enemies[ti].stats,
            true,
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
            self.hit_enemy(ti, base, skill.variance as i32)
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
        let base = logic::skill_effect(
            skill,
            &self.members[pi].stats,
            &self.members[ti].stats,
            false,
        );
        let roll = rng_next(&mut self.rng);
        let amt = logic::variance_adjust(base, skill.variance as i32, roll).max(0);
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

    /// Apply item `item_id`, used by member `pi`, to ally `target` (falling back to
    /// the user when that member is gone): restore HP and SP by the item's flat
    /// amount plus its percent-of-maximum (each clamped to the maximum) and cure
    /// each of its `cure_states` from the recipient. The item was already consumed
    /// from the inventory when the order was committed, so an unknown id or an item
    /// with no restorative effect still logs a use line.
    fn apply_item(&mut self, pi: usize, item_id: u32, target: usize) -> String {
        let user = self.members[pi].name.clone();
        let Some(item) = self.items.iter().find(|i| i.id == item_id).cloned() else {
            return format!("{user} használ");
        };
        let ti = if self.members.get(target).is_some_and(|m| m.alive()) {
            target
        } else {
            pi
        };
        let (max_hp, max_sp) = (self.members[ti].max_hp, self.members[ti].max_sp);
        let hp_gain = item.recover_hp as i32 + max_hp * item.recover_hp_rate as i32 / 100;
        let sp_gain = item.recover_sp as i32 + max_sp * item.recover_sp_rate as i32 / 100;
        if hp_gain > 0 {
            self.members[ti].hp = (self.members[ti].hp + hp_gain).min(max_hp);
        }
        if sp_gain > 0 {
            self.members[ti].sp = (self.members[ti].sp + sp_gain).min(max_sp);
        }
        let mut cured: Vec<String> = Vec::new();
        for &sid in &item.cure_states {
            if logic::has_state(&self.members[ti].states, sid) {
                logic::cure(&mut self.members[ti].states, sid);
                if let Some(state) = self.states.iter().find(|s| s.id == sid) {
                    cured.push(state.name.clone());
                }
            }
        }
        let gain = match (hp_gain > 0, sp_gain > 0) {
            (true, true) => format!(" (+{hp_gain} HP, +{sp_gain} SP)"),
            (true, false) => format!(" (+{hp_gain} HP)"),
            (false, true) => format!(" (+{sp_gain} SP)"),
            (false, false) => String::new(),
        };
        let recipient = self.members[ti].name.clone();
        let mut lines = vec![format!("{user} használ: {}{gain}", item.name)];
        for state_name in cured {
            lines.push(format!("{recipient} gyógyul: {state_name}"));
        }
        lines.join("\n")
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
    /// clamped to its max HP; any other scope damages the member (halved once if
    /// the member's equipment guards one of the skill's elements, and again while
    /// it defends, like a physical hit). `None` for an unknown skill id.
    fn enemy_cast(&mut self, ei: usize, skill_id: u32, target: usize) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id).cloned()?;
        let name = self.enemies[ei].name.clone();
        if matches!(skill.scope, 2..=4) {
            let base = logic::skill_effect(
                &skill,
                &self.enemies[ei].stats,
                &self.enemies[ei].stats,
                false,
            );
            let roll = rng_next(&mut self.rng);
            let amt = logic::variance_adjust(base, skill.variance as i32, roll).max(0);
            let e = &mut self.enemies[ei];
            e.hp = (e.hp + amt).min(e.max_hp);
            return Some(format!("{name} varázsol: {name} +{amt}"));
        }
        let ti = self.retarget_member(target)?;
        let base = logic::skill_effect(
            &skill,
            &self.enemies[ei].stats,
            &self.members[ti].stats,
            true,
        );
        // Members carry no A–E element ranks, so an enemy skill's element bites
        // only through the target's equipment: one guarded element halves it
        // (min 1). Only skills that actually carry an attribute can be resisted.
        let resisted = base > 0
            && skill
                .attributes
                .iter()
                .any(|a| self.members[ti].resist_attributes.contains(a));
        let base = if resisted { (base / 2).max(1) } else { base };
        let dmg = self.hit_member(ti, base, skill.variance as i32);
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
    /// hold turns (the death state is exempt), logging whatever lifts. Per-turn HP
    /// change (poison drain, regen) is applied separately, at the start of each
    /// battler's turn, by [`Battle::tick_state_hp`].
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
    use super::super::model::testkit::{build_1v2, build_party2};
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
            variance: 4,
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
            hp_change_type: 0,
            hp_change_max: 0,
            hp_change_val: 0,
            hp_change_map_steps: 0,
            hp_change_map_val: 0,
        }
    }

    fn hp_change_state(
        id: u32,
        hp_change_type: u32,
        hp_change_max: u32,
        hp_change_val: u32,
    ) -> amnezia_data::StateDef {
        amnezia_data::StateDef {
            id,
            name: "Méreg".into(),
            restriction: 0,
            priority: 50,
            hold_turn: 99,
            auto_release_prob: 0,
            release_by_damage: 0,
            hp_change_type,
            hp_change_max,
            hp_change_val,
            hp_change_map_steps: 0,
            hp_change_map_val: 0,
        }
    }

    #[test]
    fn poison_drains_a_fighter_and_a_foe_by_val_plus_max_percent_at_turn_start() {
        let mut battle = build_1v2();
        // Méreg (state 2): type 0 lose, 5% of max HP + 1 flat, each turn.
        battle.states = vec![hp_change_state(2, 0, 5, 1)];
        battle.members[0].states = vec![(2, 0)];
        battle.enemies[0].states = vec![(2, 0)];
        let (m_max, f_max) = (battle.members[0].max_hp, battle.enemies[0].max_hp);
        let (m_before, f_before) = (battle.members[0].hp, battle.enemies[0].hp);
        battle.tick_state_hp(Source::Party(0));
        battle.tick_state_hp(Source::Enemy(0));
        assert_eq!(m_before - battle.members[0].hp, 1 + m_max * 5 / 100);
        assert_eq!(f_before - battle.enemies[0].hp, 1 + f_max * 5 / 100);
        assert!(battle.log.iter().any(|l| l.contains("Méreg -")));
    }

    #[test]
    fn a_type_2_hp_change_state_drains_nothing() {
        let mut battle = build_1v2();
        // Type 2 = nothing, even with a large configured amount.
        battle.states = vec![hp_change_state(2, 2, 50, 10)];
        battle.members[0].states = vec![(2, 0)];
        let before = battle.members[0].hp;
        battle.tick_state_hp(Source::Party(0));
        assert_eq!(battle.members[0].hp, before);
    }

    #[test]
    fn poison_can_reduce_a_battler_to_zero_and_kill_it() {
        let mut battle = build_1v2();
        // A 100%-of-max drain empties the fighter's HP outright.
        battle.states = vec![hp_change_state(2, 0, 100, 0)];
        battle.members[0].states = vec![(2, 0)];
        battle.tick_state_hp(Source::Party(0));
        assert_eq!(battle.members[0].hp, 0);
        assert!(!battle.members[0].alive());
    }

    #[test]
    fn a_type_1_hp_change_state_regenerates_capped_at_max() {
        let mut battle = build_1v2();
        battle.states = vec![hp_change_state(2, 1, 10, 5)]; // gain 5 + 10% of max
        let max = battle.members[0].max_hp;
        battle.members[0].hp = 10;
        battle.members[0].states = vec![(2, 0)];
        battle.tick_state_hp(Source::Party(0));
        assert_eq!(battle.members[0].hp, (10 + 5 + max * 10 / 100).min(max));
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
        // var=4 spreads the tripled base by up to ±20%; the crit clearly beats a
        // plain blow either way.
        let tripled = base * 3;
        assert!(dmg >= tripled - tripled * 2 / 10 && dmg <= tripled + tripled * 2 / 10 + 1);
        assert!(dmg > base);
    }

    #[test]
    fn a_missed_strike_deals_no_damage() {
        let mut battle = build_1v2();
        battle.members[0].weapon_hit = 90; // bare-hands default
        // The effective chance is the base hit adjusted by the agility gap; wind
        // the rng to a state whose next to-hit roll falls in that miss band.
        let hit = logic::to_hit(
            logic::effective_hit(battle.members[0].weapon_hit),
            battle.members[0].stats.agility,
            battle.enemies[0].stats.agility,
        );
        loop {
            let mut probe = battle.rng;
            if (rng_next(&mut probe) % 100) as i32 >= hit {
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
        // base 8 with var=4 spreads to [7,10], halved by defence to [3,5].
        battle.hit_member(0, 8, 4);
        let taken = full - battle.members[0].hp;
        assert!((3..=5).contains(&taken));
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
        let plain = logic::skill_effect(
            &battle.skills[0],
            &battle.members[0].stats,
            &battle.enemies[0].stats,
            true,
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
            hp_change_type: 0,
            hp_change_max: 0,
            hp_change_val: 0,
            hp_change_map_steps: 0,
            hp_change_map_val: 0,
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
            hp_change_type: 0,
            hp_change_max: 0,
            hp_change_val: 0,
            hp_change_map_steps: 0,
            hp_change_map_val: 0,
        }
    }

    #[test]
    fn being_hit_wears_off_a_damage_release_state_but_never_death() {
        let mut battle = build_1v2();
        // id 1 is the death state (exempt); id 2 shakes off on any hit.
        battle.states = vec![poison_state(1), damage_release_state(2)];
        battle.members[0].states = vec![(1, 0), (2, 0)];
        battle.hit_member(0, 8, 4);
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

    fn medicine(
        id: u32,
        recover_hp: u32,
        recover_sp: u32,
        cure_states: Vec<u32>,
    ) -> amnezia_data::ItemDef {
        amnezia_data::ItemDef {
            id,
            name: "Gyógyfű".into(),
            description: String::new(),
            item_type: 6,
            price: 0,
            recover_hp,
            recover_hp_rate: 0,
            recover_sp,
            recover_sp_rate: 0,
            cure_states,
            scope: 0,
            only_field: false,
            uses: 0,
            atk: 0,
            def: 0,
            spi: 0,
            agi: 0,
            attribute_defense: vec![],
            state_defense: vec![],
            two_handed: false,
            hit: 0,
            crit: 0,
            weapon_animation: 0,
        }
    }

    #[test]
    fn a_recover_hp_item_raises_the_users_hp_clamped_to_max() {
        let mut battle = build_1v2();
        let max = battle.members[0].max_hp;
        // A flat +20 raises a wounded user by exactly 20.
        battle.items = vec![medicine(50, 20, 0, vec![])];
        battle.members[0].hp = 10;
        let line = battle.apply_item(0, 50, 0);
        assert_eq!(battle.members[0].hp, 30);
        assert!(line.contains("+20 HP"));
        // A heal that overshoots the maximum clamps to it.
        battle.members[0].hp = max - 5;
        battle.items = vec![medicine(51, 200, 0, vec![])];
        battle.apply_item(0, 51, 0);
        assert_eq!(battle.members[0].hp, max);
    }

    #[test]
    fn a_cure_states_item_lifts_that_state_from_the_user() {
        let mut battle = build_1v2();
        battle.states = vec![poison_state(3)];
        battle.members[0].states = vec![(3, 0)];
        battle.items = vec![medicine(60, 0, 0, vec![3])];
        let line = battle.apply_item(0, 60, 0);
        assert!(!logic::has_state(&battle.members[0].states, 3));
        assert!(line.contains("gyógyul"));
    }

    #[test]
    fn an_item_used_on_an_ally_heals_that_member_not_the_caster() {
        let mut battle = build_party2();
        battle.items = vec![medicine(50, 20, 0, vec![])];
        let max = battle.members[1].max_hp;
        battle.members[1].hp = (max - 25).max(0);
        let before_ally = battle.members[1].hp;
        let before_caster = battle.members[0].hp;
        let line = battle.apply_item(0, 50, 1); // caster 0 uses the item on ally 1
        assert_eq!(battle.members[1].hp, (before_ally + 20).min(max));
        assert_eq!(battle.members[0].hp, before_caster); // the caster is untouched
        assert!(line.contains("+20 HP"));
    }

    /// An always-eligible enemy AI entry of the given `basic` code.
    fn enemy_action_def(basic: u32) -> amnezia_data::EnemyActionDef {
        amnezia_data::EnemyActionDef {
            kind: 0,
            basic,
            skill_id: 0,
            enemy_id: 0,
            condition_type: 0, // always eligible
            condition_min: 0,
            condition_max: 0,
            priority: 1,
        }
    }

    #[test]
    fn a_defending_foe_takes_half_of_an_identical_strike() {
        let mut battle = build_1v2();
        battle.members[0].weapon_hit = 100; // never miss
        battle.members[0].weapon_crit = 0; // never crit — isolate the halving
        // Foe 0 (open) and foe 1 (defending) are identical bandits; strike each
        // from the same RNG state so only the Defend stance differs.
        let rng_save = battle.rng;
        let Strike::Hit { dmg: full, .. } = battle.strike_enemy(0, 0) else {
            panic!("a forced-hit strike missed");
        };
        battle.rng = rng_save;
        battle.enemies[1].defending = true;
        let Strike::Hit { dmg: half, .. } = battle.strike_enemy(0, 1) else {
            panic!("a forced-hit strike missed");
        };
        assert!(half < full);
        assert_eq!(half, (full / 2).max(1));
    }

    #[test]
    fn a_defending_enemy_guards_and_deals_no_damage_that_turn() {
        let mut battle = build_1v2();
        for e in &mut battle.enemies {
            e.actions = vec![enemy_action_def(2)]; // basic 2 = defend
        }
        let hp_before = battle.members[0].hp;
        battle.commit(Command::Defend); // the lone member defends -> resolution
        while battle.resolve_next() {}
        assert!(battle.enemies.iter().all(|e| e.defending));
        assert_eq!(battle.members[0].hp, hp_before); // no foe attacked
        assert!(battle.log.iter().any(|l| l.contains("védekezik")));
    }

    /// Advance `battle.rng` until the enemy to-hit rolls at the given draw
    /// `offsets` all land (roll % 100 < 50, comfortably under the agility-adjusted
    /// enemy hit here), so a strike-count comparison is not spoiled by a miss.
    fn wind_enemy_hits(battle: &mut Battle, offsets: &[usize]) {
        let span = offsets.iter().copied().max().map_or(0, |m| m + 1);
        loop {
            let mut probe = battle.rng;
            let draws: Vec<u64> = (0..span).map(|_| rng_next(&mut probe)).collect();
            if offsets.iter().all(|&o| draws[o] % 100 < 50) {
                return;
            }
            rng_next(&mut battle.rng);
        }
    }

    #[test]
    fn a_double_attack_strikes_the_target_twice() {
        let mut battle = build_1v2();
        let hp0 = battle.members[0].hp;
        // Land both to-hit rolls (draw 0 for the single blow, draws 0 and 2 for
        // the double) so the comparison reflects strike count, not a chance miss.
        wind_enemy_hits(&mut battle, &[0, 2]);
        let rng_save = battle.rng;
        // Baseline: a single enemy strike from this RNG state.
        battle.apply(Action {
            source: Source::Enemy(0),
            kind: Command::Attack { target: 0 },
            agility: 0,
        });
        let single = hp0 - battle.members[0].hp;
        assert!(single > 0);
        // Same RNG state, but a double-attack: the first blow matches `single`,
        // the second adds more, so the total clearly exceeds one strike.
        battle.rng = rng_save;
        battle.members[0].hp = hp0;
        battle.apply(Action {
            source: Source::Enemy(0),
            kind: Command::DoubleAttack { target: 0 },
            agility: 0,
        });
        let double = hp0 - battle.members[0].hp;
        assert!(
            double > single,
            "double-attack ({double}) should exceed a single strike ({single})"
        );
    }

    #[test]
    fn self_destruct_hits_every_member_then_kills_the_foe() {
        let mut battle = build_party2(); // 2 members, 1 foe (attack 20)
        let hp = [battle.members[0].hp, battle.members[1].hp];
        battle.apply(Action {
            source: Source::Enemy(0),
            kind: Command::SelfDestruct,
            agility: 0,
        });
        assert!(battle.members[0].hp < hp[0]);
        assert!(battle.members[1].hp < hp[1]);
        assert_eq!(battle.enemies[0].hp, 0);
        assert!(!battle.enemies[0].alive());
        assert!(battle.log.iter().any(|l| l.contains("felrobban")));
    }

    #[test]
    fn an_escaping_foe_leaves_battle_and_grants_no_reward() {
        let mut battle = build_1v2(); // 2 foes, each 10 exp / 30 gold
        battle.apply(Action {
            source: Source::Enemy(0),
            kind: Command::Escape,
            agility: 0,
        });
        assert!(battle.enemies[0].fled);
        assert!(!battle.enemies[0].alive());
        assert!(!battle.living_enemies().contains(&0));
        // Defeat the remaining foe: victory pays only for the one truly beaten.
        battle.enemies[1].hp = 0;
        assert!(matches!(battle.end_state(), Some(BattleOutcome::Victory)));
        assert_eq!(battle.victory_rewards(), (10, 30));
        assert!(battle.log.iter().any(|l| l.contains("elmenekül")));
    }

    #[test]
    fn a_charged_foe_doubles_its_next_strike_then_clears() {
        let mut battle = build_1v2();
        let hp0 = battle.members[0].hp;
        wind_enemy_hits(&mut battle, &[0]); // land the single to-hit roll
        let rng_save = battle.rng;
        battle.apply(Action {
            source: Source::Enemy(0),
            kind: Command::Attack { target: 0 },
            agility: 0,
        });
        let normal = hp0 - battle.members[0].hp;
        assert!(normal > 0);
        // Same RNG, but the foe has charged: the strike lands double, then clears.
        battle.rng = rng_save;
        battle.members[0].hp = hp0;
        battle.enemies[0].charging = true;
        battle.apply(Action {
            source: Source::Enemy(0),
            kind: Command::Attack { target: 0 },
            agility: 0,
        });
        let charged = hp0 - battle.members[0].hp;
        assert!(
            charged > normal,
            "charged strike ({charged}) should exceed a normal one ({normal})"
        );
        assert!(!battle.enemies[0].charging); // consumed by the strike
    }

    #[test]
    fn equipped_element_defence_halves_a_matching_enemy_skill_only() {
        use super::super::model::testkit;
        use crate::progression::Progression;
        use crate::vitals::Vitals;
        let mut ron = testkit::actor(1, 3, 200, 50);
        ron.armor = 2; // equip armor guarding attribute 5
        let actors = vec![&ron];
        let items = vec![testkit::item(2, 0, 0, 0, 0, 5)];
        let monsters = vec![testkit::monster(1, 30, 10, 30)];
        let troop = testkit::troop(&[(1, 100, 100)]);
        let mut battle = Battle::build(
            &troop,
            &monsters,
            &actors,
            &items,
            &[],
            &[],
            &[],
            &Vitals::default(),
            &Progression::default(),
            "Cave1".into(),
            5,
        );
        assert!(battle.members[0].resist_attributes.contains(&5));
        // With the skill's variance set to 0 the halving is exact: the guarded
        // cast deals base/2 and the unguarded cast deals the full base, so the two
        // compare cleanly without depending on the variance draw.
        let before = battle.members[0].hp;
        // A guarded (attribute 5) enemy skill is halved (before variance).
        let mut guarded = damage_skill(1, 40, vec![5], vec![]);
        guarded.variance = 0;
        battle.skills = vec![guarded];
        battle.enemy_cast(0, 1, 0);
        let resisted = before - battle.members[0].hp;
        // The same skill on an unguarded element (6) lands full.
        battle.members[0].hp = before;
        let mut unguarded = damage_skill(1, 40, vec![6], vec![]);
        unguarded.variance = 0;
        battle.skills = vec![unguarded];
        battle.enemy_cast(0, 1, 0);
        let full = before - battle.members[0].hp;
        assert!(resisted < full);
        assert_eq!(resisted, (full / 2).max(1));
    }
}
