//! Skill casts: a party member's cast (damage or heal, single or multi-target),
//! the enemy-side cast, and the screen anchors each animates over.

use super::*;

impl Battle {
    /// Resolve member `pi`'s cast of skill `skill_id` at `target`: deduct SP, then
    /// apply the scoped effect — an elemental attack that may inflict states
    /// (enemy scope 0/1), or a heal that may cure them (ally scope 2/3/4). Returns
    /// the joined log line(s), or `None` for an unknown skill id (no effect).
    pub(in crate::battle::resolve) fn cast_skill(
        &mut self,
        pi: usize,
        skill_id: u32,
        target: usize,
    ) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id).cloned()?;
        self.members[pi].sp = (self.members[pi].sp - skill.sp_cost as i32).max(0);
        // Deferred casts already queued the animation before their hold.
        if !self.suppress_anim {
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

    /// Land `skill` from caster `pi` on enemy `ti`: elemental damage against the
    /// foe's resistance ranks (SP drains find no pool on a foe), an optional
    /// life-absorb for the caster, and a status-infliction roll per affected
    /// state, weighted by the foe's affliction rank. The skill's battle animation
    /// is queued once for the whole cast by [`Battle::cast_skill`], not here.
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
        let dealt = if skill.affect_hp {
            self.hit_enemy(ti, base, skill.variance as i32)
        } else {
            0
        };
        if skill.absorb && dealt > 0 {
            let f = &mut self.members[pi];
            f.hp = (f.hp + dealt).min(f.max_hp);
            let pos = (self.party_anim_x(pi), PARTY_ANIM_Y);
            self.push_number(pos, dealt.to_string(), NumberKind::Heal);
        }
        let head = if crit {
            format!("{caster} varázsol: Kritikus! {target} -{dealt}")
        } else {
            format!("{caster} varázsol: {target} -{dealt}")
        };
        let mut lines = vec![head];
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

    /// Heal ally `ti` for caster `pi`'s `skill`: restore SP or HP (clamped to the
    /// maximum), then cure each of the skill's affected states from that ally. The
    /// skill's battle animation is queued once for the whole cast by
    /// [`Battle::cast_skill`], not here.
    pub(in crate::battle::resolve) fn skill_heal_ally(
        &mut self,
        pi: usize,
        ti: usize,
        skill: &SkillDef,
    ) -> Vec<String> {
        if !self.ally_skill_target(ti, skill) {
            return Vec::new();
        }
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
        let was_dead = !f.alive();
        let old_hp = f.hp.max(0);
        let old_sp = f.sp;
        if skill.affect_sp {
            f.sp = (f.sp + amt).min(f.max_sp);
        }
        let hp_gain = if skill.affect_hp {
            amt
        } else if was_dead {
            // RPG_RT treats revival power as a percentage when HP is unchecked.
            f.max_hp * amt / 100
        } else {
            0
        };
        f.hp = (old_hp + hp_gain).min(f.max_hp);
        if was_dead {
            f.hp = f.hp.max(1);
        }
        let shown = (f.hp - old_hp).max(f.sp - old_sp);
        if shown > 0 {
            let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
            self.push_number(pos, shown.to_string(), NumberKind::Heal);
        }
        let mut lines = vec![format!("{caster} varázsol: {target} +{shown}")];
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

    /// Resolve enemy `ei`'s cast of `skill_id` at member `target`, queuing the
    /// skill's battle animation on whichever side it resolves against: an
    /// ally-scope skill (scope 2/3/4) heals the caster itself (a foe keeps no
    /// ally list) and animates on that foe, clamped to its max HP; any other
    /// scope damages the member (halved once if the member's equipment guards one
    /// of the skill's elements, and again while it defends, like a physical hit)
    /// and animates at that member's party slot. `None` for an unknown skill id.
    pub(in crate::battle::resolve) fn enemy_cast(
        &mut self,
        ei: usize,
        skill_id: u32,
        target: usize,
    ) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id).cloned()?;
        let name = self.enemies[ei].name.clone();
        if !self.suppress_anim {
            let anchors = self.enemy_skill_anim_anchors(ei, &skill, target);
            self.push_anim(skill.animation_id, anchors);
        }
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
            if amt > 0 {
                let pos = self.foe_anim_pos(ei);
                self.push_number(pos, amt.to_string(), NumberKind::Heal);
            }
            return Some(format!("{name} varázsol: {name} +{amt}"));
        }
        let ti = self.retarget_member(target)?;
        // Enemies carry no critical-hit rate in the converted model.
        let can_act = logic::worst_restriction(&self.members[ti].states, &self.states) != 1;
        let hit = logic::skill_to_hit(
            &skill,
            self.enemies[ei].stats.agility,
            self.members[ti].stats.agility,
            can_act,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
            let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
            self.pending_se.push(BattleSe::Dodge);
            self.push_number(pos, "Miss".to_string(), NumberKind::Miss);
            return Some(format!(
                "{name} varázsol: {} elkerülte",
                self.members[ti].name
            ));
        }
        let base = logic::skill_effect(
            &skill,
            &self.enemies[ei].stats,
            &self.members[ti].stats,
            true,
        );
        // Actor resistance ranks are not converted; equipment supplies resistance.
        let resisted = base > 0
            && skill
                .attributes
                .iter()
                .any(|a| self.members[ti].resist_attributes.contains(a));
        let base = if resisted { (base / 2).max(1) } else { base };
        let dmg = self.hit_member(ti, base, skill.variance as i32);
        Some(format!("{name} varázsol: {} -{dmg}", self.members[ti].name))
    }

    /// The screen anchors an enemy `ei`'s cast of `skill` at member `target`
    /// animates over: the casting foe itself for an ally-scope skill (scope 2–4,
    /// which a foe turns on itself), else the targeted member's party slot
    /// (falling back to any living member). Draws no RNG.
    pub(in crate::battle::resolve) fn enemy_skill_anim_anchors(
        &self,
        ei: usize,
        skill: &SkillDef,
        target: usize,
    ) -> Vec<(f32, f32)> {
        if matches!(skill.scope, 2..=4) {
            vec![self.foe_anim_pos(ei)]
        } else {
            let ti = if self.members.get(target).is_some_and(|m| m.alive()) {
                Some(target)
            } else {
                self.living_members().first().copied()
            };
            ti.map(|ti| vec![(self.party_anim_x(ti), PARTY_ANIM_Y)])
                .unwrap_or_default()
        }
    }
}
