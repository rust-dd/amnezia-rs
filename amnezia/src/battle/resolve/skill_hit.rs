use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn skill_hit_battler(
        &mut self,
        source: Source,
        target: Source,
        skill: &SkillDef,
    ) -> Vec<String> {
        let name = self.battler_name(target).to_string();
        let ally = matches!(target, Source::Party(_));
        let hit = self.skill_hit_chance(source, target, skill);
        let effect = self.skill_magnitude(source, target, skill);
        let mut lines = Vec::new();
        let mut success = false;
        if skill.affect_hp && self.skill_roll(hit) {
            let old_hp = self.battler_hp(target).max(0);
            let release_rate = if skill.absorb {
                0
            } else {
                skill.physical_rate * 10
            };
            let dealt = match target {
                Source::Party(i) => self.hit_member(i, effect, 0, release_rate),
                Source::Enemy(i) => self.hit_enemy(i, effect, 0, release_rate),
            };
            if skill.absorb && dealt > 0 {
                let absorbed = self.restore_skill_hp(source, dealt.min(old_hp));
                self.report_hit(
                    self.battler_pos(source),
                    absorbed.to_string(),
                    HitKind::Heal,
                );
            }
            success = !skill.absorb || dealt.min(old_hp) > 0;
            if skill.absorb {
                if dealt > 0 {
                    lines.push(self.text.absorbed(&name, ally, true, dealt.min(old_hp)));
                }
            } else {
                lines.push(self.text.damaged(&name, ally, dealt));
            }
        }
        if self.battler_hp(target) <= 0 {
            return lines;
        }
        if skill.affect_sp && self.skill_roll(hit) {
            let lost = self.skill_sp_damage(source, target, effect, skill.absorb);
            success |= lost > 0;
            if skill.absorb && lost > 0 {
                lines.push(self.text.absorbed(&name, ally, false, lost));
            } else if lost > 0 {
                lines.push(
                    self.text
                        .parameter_changed(&name, &self.text.spirit_points, -lost),
                );
            }
        }
        if success || !(skill.affect_hp || skill.affect_sp) {
            let stats = self.skill_stat_effects(target, skill, hit, effect);
            success |= !stats.is_empty();
            lines.extend(stats);
            let states = self.skill_states(target, skill, hit);
            success |= states.success;
            lines.extend(states.lines);
        }
        if !success {
            self.pending_se.push(BattleSe::Dodge);
            self.report_hit(self.battler_pos(target), "Miss".to_string(), HitKind::Miss);
            lines.push(self.text.skill_failed(&name, skill.failure_message));
        }
        lines
    }

    pub(in crate::battle::resolve) fn skill_magnitude(
        &mut self,
        source: Source,
        target: Source,
        skill: &SkillDef,
    ) -> i32 {
        let base = logic::skill_effect(
            skill,
            &self.battler_stats(source),
            &self.battler_stats(target),
            skill.scope < 2,
        );
        let base = self.battler_attribute_damage(base, target, &skill.attributes);
        if skill.variance == 0 || base <= 0 {
            base
        } else {
            logic::variance_adjust(base, skill.variance as i32, rng_next(&mut self.rng)).max(0)
        }
    }

    pub(in crate::battle::resolve) fn restore_skill_hp(
        &mut self,
        target: Source,
        amount: i32,
    ) -> i32 {
        let (hp, max) = match target {
            Source::Party(i) => {
                let member = &mut self.members[i];
                (&mut member.hp, member.max_hp)
            }
            Source::Enemy(i) => {
                let enemy = &mut self.enemies[i];
                (&mut enemy.hp, enemy.max_hp)
            }
        };
        let before = *hp;
        *hp = hp.saturating_add(amount).clamp(0, max);
        *hp - before
    }
}
