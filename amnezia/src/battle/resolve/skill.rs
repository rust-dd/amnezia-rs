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
            self.push_skill_anim(Source::Party(pi), &skill, target);
        }
        let mut lines: Vec<String> = Vec::new();
        match skill.scope {
            1 => {
                let foes = self.living_enemies();
                // Each target's result has its own message beat.
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

    pub(in crate::battle::resolve) fn skill_hit_enemy(
        &mut self,
        pi: usize,
        ti: usize,
        skill: &SkillDef,
    ) -> Vec<String> {
        self.skill_hit_battler(Source::Party(pi), Source::Enemy(ti), skill)
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
