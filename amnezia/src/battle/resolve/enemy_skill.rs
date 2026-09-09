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
            let anchors = self.enemy_skill_anim_anchors(ei, &skill, target);
            self.push_anim(skill.animation_id, anchors);
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
            self.enemy_skill_hit_member(ei, ti, skill)
                .into_iter()
                .collect()
        } else {
            Vec::new()
        }
    }

    fn enemy_skill_hit_member(&mut self, ei: usize, ti: usize, skill: &SkillDef) -> Option<String> {
        let name = self.enemies[ei].name.clone();
        let can_act = logic::worst_restriction(&self.members[ti].states, &self.states) != 1;
        let hit = logic::skill_to_hit(
            skill,
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
            skill,
            &self.enemies[ei].stats,
            &self.members[ti].stats,
            true,
        );
        let resisted = base > 0
            && skill
                .attributes
                .iter()
                .any(|a| self.members[ti].resist_attributes.contains(a));
        let base = if resisted { (base / 2).max(1) } else { base };
        let effect =
            logic::variance_adjust(base, skill.variance as i32, rng_next(&mut self.rng)).max(0);
        let old_hp = self.members[ti].hp.max(0);
        let dmg = if skill.affect_hp {
            self.hit_member(ti, effect, 0)
        } else {
            0
        };
        if skill.absorb && dmg > 0 {
            let enemy = &mut self.enemies[ei];
            enemy.hp = (enemy.hp + dmg.min(old_hp)).min(enemy.max_hp);
        }
        let sp_lost = if skill.affect_sp && self.members[ti].alive() {
            self.skill_sp_damage(Source::Enemy(ei), Source::Party(ti), effect, skill.absorb)
        } else {
            0
        };
        let mut line = if skill.affect_sp && !skill.affect_hp {
            format!("{name} varázsol: {} -{sp_lost} SP", self.members[ti].name)
        } else {
            format!("{name} varázsol: {} -{dmg}", self.members[ti].name)
        };
        if skill.affect_sp && skill.affect_hp {
            line.push_str(&format!("; -{sp_lost} SP"));
        }
        Some(line)
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
