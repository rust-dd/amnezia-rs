use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn enemy_cast(
        &mut self,
        ei: usize,
        skill_id: u32,
        target: usize,
    ) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id).cloned()?;
        let target = self.enemy_skill_target_index(&skill, target)?;
        if !self.suppress_anim {
            if !self.pay_skill(Source::Enemy(ei), &skill) {
                return None;
            }
            self.push_skill_anim(Source::Enemy(ei), &skill, target);
        }
        let targets = self.enemy_skill_targets(ei, &skill, target);
        let mut targets = targets.into_iter();
        let mut lines = Vec::new();
        if let Some(first) = targets.next() {
            lines.extend(self.enemy_skill_target(ei, first, &skill));
        }
        for target in targets {
            self.steps.push_back(Step::EnemySkillTarget {
                ei,
                target,
                skill_id,
            });
        }
        Some(if lines.is_empty() {
            format!("{} varázsol", self.enemies[ei].name)
        } else {
            lines.join("\n")
        })
    }

    pub(in crate::battle::resolve) fn enemy_skill_target(
        &mut self,
        ei: usize,
        ti: usize,
        skill: &SkillDef,
    ) -> Vec<String> {
        if matches!(skill.scope, 2..=4) {
            if self.foe_skill_target(ti, skill) {
                self.skill_heal_battler(Source::Enemy(ei), Source::Enemy(ti), skill)
            } else {
                Vec::new()
            }
        } else if self.members.get(ti).is_some_and(|member| member.alive()) {
            self.skill_hit_battler(Source::Enemy(ei), Source::Party(ti), skill)
        } else {
            Vec::new()
        }
    }

    fn foe_skill_target(&self, i: usize, skill: &SkillDef) -> bool {
        self.enemies.get(i).is_some_and(|enemy| {
            !enemy.fled && (enemy.alive() || skill.affected_states.contains(&1))
        })
    }

    pub(in crate::battle::resolve) fn enemy_skill_target_index(
        &mut self,
        skill: &SkillDef,
        target: usize,
    ) -> Option<usize> {
        if skill.scope == 0 {
            return self.retarget_member(target);
        }
        if skill.scope == 3 && !self.foe_skill_target(target, skill) {
            let valid = (0..self.enemies.len())
                .map(|i| self.foe_skill_target(i, skill))
                .collect::<Vec<_>>();
            return logic::select_target(&valid, rng_next(&mut self.rng) as usize);
        }
        Some(target)
    }

    fn enemy_skill_targets(&self, ei: usize, skill: &SkillDef, target: usize) -> Vec<usize> {
        match skill.scope {
            1 => self.living_members(),
            2 => vec![ei],
            3 => {
                if self.foe_skill_target(target, skill) {
                    vec![target]
                } else {
                    (0..self.enemies.len())
                        .find(|&i| self.foe_skill_target(i, skill))
                        .into_iter()
                        .collect()
                }
            }
            4 => (0..self.enemies.len())
                .filter(|&i| self.foe_skill_target(i, skill))
                .collect(),
            _ => {
                if self
                    .members
                    .get(target)
                    .is_some_and(|member| member.alive())
                {
                    vec![target]
                } else {
                    self.living_members().first().copied().into_iter().collect()
                }
            }
        }
    }

    pub(in crate::battle::resolve) fn enemy_skill_anim_anchors(
        &self,
        ei: usize,
        skill: &SkillDef,
        target: usize,
    ) -> Vec<(f32, f32)> {
        self.enemy_skill_targets(ei, skill, target)
            .into_iter()
            .map(|ti| {
                if matches!(skill.scope, 2..=4) {
                    self.foe_anim_pos(ti)
                } else {
                    (self.party_anim_x(ti), PARTY_ANIM_Y)
                }
            })
            .collect()
    }
}
