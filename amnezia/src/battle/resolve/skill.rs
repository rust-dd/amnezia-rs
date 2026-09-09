//! Party-side skill targeting, animation anchors, and effects.

use super::*;

impl Battle {
    /// Resolve member `pi`'s cast of skill `skill_id` at `target`: deduct SP, then
    /// apply the scoped effect — an elemental attack that may inflict states
    /// (enemy scope 0/1), or a heal that may cure them (ally scope 2/3/4). Returns
    /// the joined log lines, or `None` for an unknown or unaffordable skill.
    pub(in crate::battle::resolve) fn cast_skill(
        &mut self,
        pi: usize,
        skill_id: u32,
        target: usize,
    ) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id).cloned()?;
        // Deferred casts already queued the animation before their hold.
        if !self.suppress_anim {
            if !self.pay_skill(Source::Party(pi), &skill) {
                return None;
            }
            let anchors = self.skill_anim_anchors(pi, &skill, target);
            self.push_anim(skill.animation_id, anchors);
        }
        let mut lines: Vec<String> = Vec::new();
        match skill.scope {
            1 => {
                let foes = self.living_enemies();
                // Separate ticks keep multi-target damage numbers readable.
                let mut targets = foes.into_iter();
                if let Some(first) = targets.next() {
                    lines.extend(self.skill_hit_enemy(pi, first, &skill));
                }
                for ti in targets {
                    self.steps.push_back(Step::HitEnemy {
                        pi,
                        ti,
                        skill_id: skill.id,
                    });
                }
            }
            2 => {
                lines.extend(self.skill_heal_ally(pi, pi, &skill));
            }
            3 => {
                if self.ally_skill_target(target, &skill) {
                    lines.extend(self.skill_heal_ally(pi, target, &skill));
                }
            }
            4 => {
                let allies = self.ally_skill_targets(&skill);
                let mut targets = allies.into_iter();
                if let Some(first) = targets.next() {
                    lines.extend(self.skill_heal_ally(pi, first, &skill));
                }
                for ti in targets {
                    self.steps.push_back(Step::HealAlly {
                        pi,
                        ti,
                        skill_id: skill.id,
                    });
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

    /// The screen anchors party member `pi`'s cast of `skill` at `target`
    /// animates over: every living foe for an all-enemy skill (scope 1), the
    /// caster's own party slot for a self-heal (scope 2), the chosen ally's slot
    /// for a single-ally heal (scope 3), every applicable ally for a party heal
    /// (scope 4, including fallen allies for revival), or the targeted foe otherwise
    /// (falling back to any living foe if that one has fallen). Draws no RNG, so it
    /// can be computed up front to queue the animation before the hold.
    pub(in crate::battle::resolve) fn skill_anim_anchors(
        &self,
        pi: usize,
        skill: &SkillDef,
        target: usize,
    ) -> Vec<(f32, f32)> {
        match skill.scope {
            1 => self
                .living_enemies()
                .iter()
                .map(|&ti| self.foe_anim_pos(ti))
                .collect(),
            2 => vec![(self.party_anim_x(pi), PARTY_ANIM_Y)],
            3 => {
                if self.ally_skill_target(target, skill) {
                    vec![(self.party_anim_x(target), PARTY_ANIM_Y)]
                } else {
                    Vec::new()
                }
            }
            4 => self
                .ally_skill_targets(skill)
                .iter()
                .map(|&ti| (self.party_anim_x(ti), PARTY_ANIM_Y))
                .collect(),
            _ => {
                let ti = if self.enemies.get(target).is_some_and(|e| e.alive()) {
                    Some(target)
                } else {
                    self.living_enemies().first().copied()
                };
                ti.map(|ti| vec![self.foe_anim_pos(ti)]).unwrap_or_default()
            }
        }
    }

    /// Apply one target's damage, absorption, and state effects after the cast animation.
    pub(in crate::battle::resolve) fn skill_hit_enemy(
        &mut self,
        pi: usize,
        ti: usize,
        skill: &SkillDef,
    ) -> Vec<String> {
        let caster = self.members[pi].name.clone();
        let target = self.enemies[ti].name.clone();
        let can_act = logic::worst_restriction(&self.enemies[ti].states, &self.states) != 1;
        let hit = logic::skill_to_hit(
            skill,
            self.members[pi].stats.agility,
            self.enemies[ti].stats.agility,
            can_act,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
            let pos = self.foe_anim_pos(ti);
            self.pending_se.push(BattleSe::Dodge);
            self.push_number(pos, "Miss".to_string(), NumberKind::Miss);
            return vec![format!("{caster} varázsol: {target} elkerülte")];
        }
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
        let crit = ((rng_next(&mut self.rng) % 100) as u32) < self.members[pi].weapon_crit;
        let base = if crit {
            logic::critical_damage(base)
        } else {
            base
        };
        let effect =
            logic::variance_adjust(base, skill.variance as i32, rng_next(&mut self.rng)).max(0);
        let old_hp = self.enemies[ti].hp.max(0);
        let dealt = if skill.affect_hp {
            self.hit_enemy(ti, effect, 0)
        } else {
            0
        };
        if skill.absorb && dealt > 0 {
            let f = &mut self.members[pi];
            let absorbed = dealt.min(old_hp);
            f.hp = (f.hp + absorbed).min(f.max_hp);
            let pos = (self.party_anim_x(pi), PARTY_ANIM_Y);
            self.push_number(pos, absorbed.to_string(), NumberKind::Heal);
        }
        let sp_lost = if skill.affect_sp && self.enemies[ti].alive() {
            self.skill_sp_damage(Source::Party(pi), Source::Enemy(ti), effect, skill.absorb)
        } else {
            0
        };
        let head = if skill.affect_sp && !skill.affect_hp {
            format!("{caster} varázsol: {target} -{sp_lost} SP")
        } else if crit {
            format!("{caster} varázsol: Kritikus! {target} -{dealt}")
        } else {
            format!("{caster} varázsol: {target} -{dealt}")
        };
        let mut lines = vec![head];
        if skill.affect_sp && skill.affect_hp {
            lines.push(format!("{target} -{sp_lost} SP"));
        }
        for &sid in &skill.affected_states {
            let rank = sid
                .checked_sub(1)
                .and_then(|i| self.enemies[ti].state_ranks.get(i as usize).copied())
                .unwrap_or(2);
            if ((rng_next(&mut self.rng) % 100) as u32) < logic::state_infliction_chance(rank) {
                logic::inflict(&mut self.enemies[ti].states, sid);
                if sid == 1 {
                    self.enemies[ti].hp = 0;
                    self.start_foe_death(ti, false);
                }
                if let Some(state) = self.states.iter().find(|s| s.id == sid) {
                    lines.push(format!("{target} státusz: {}", state.name));
                }
            }
        }
        lines
    }

    pub(in crate::battle::resolve) fn skill_heal_ally(
        &mut self,
        pi: usize,
        ti: usize,
        skill: &SkillDef,
    ) -> Vec<String> {
        if self.ally_skill_target(ti, skill) {
            self.skill_heal_battler(Source::Party(pi), Source::Party(ti), skill)
        } else {
            Vec::new()
        }
    }

    pub(in crate::battle::resolve) fn ally_skill_target(
        &self,
        target: usize,
        skill: &SkillDef,
    ) -> bool {
        self.members
            .get(target)
            .is_some_and(|m| m.alive() || skill.affected_states.contains(&1))
    }

    fn ally_skill_targets(&self, skill: &SkillDef) -> Vec<usize> {
        (0..self.members.len())
            .filter(|&target| self.ally_skill_target(target, skill))
            .collect()
    }
}
