use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn plan_skill(
        &mut self,
        source: Source,
        target: Source,
        skill: &SkillDef,
    ) -> TargetPlan {
        let positive = matches!(skill.scope, 2..=4);
        let mut plan = TargetPlan {
            positive,
            failure: skill.failure_message,
            ..Default::default()
        };
        let dead = self.battler_hp(target) <= 0;
        if dead && !(positive && skill.affected_states.contains(&1)) {
            return plan;
        }
        let amount = self.skill_magnitude(source, target, skill);
        let hit = self.skill_hit_chance(source, target, skill);
        let absorb = skill.absorb && !positive;
        let mut states = self.planning_states(target);
        if skill.affect_hp && self.skill_roll(hit) {
            if positive {
                if amount != 0 {
                    plan.hp = Some(amount);
                    plan.success = true;
                }
            } else {
                let damage = if self.defending(target) {
                    logic::defended(amount)
                } else {
                    amount
                };
                let damage = if absorb {
                    damage.min(self.battler_hp(target))
                } else {
                    damage
                };
                if !absorb || damage > 0 {
                    plan.hp = Some(-damage);
                    plan.absorb_hp = absorb;
                    plan.success = true;
                    if !absorb && self.battler_hp(target) > damage {
                        self.plan_releases(&mut plan, &mut states, skill.physical_rate * 10);
                    }
                }
            }
        }
        if !dead && self.battler_hp(target) + plan.hp.unwrap_or(0) <= 0 {
            return plan;
        }
        let delta = if positive { amount } else { -amount };
        if skill.affect_sp && self.skill_roll(hit) {
            let (_, sp, max) = self.pools(target);
            plan.sp = (sp + delta).clamp(0, max) - sp;
            plan.absorb_sp = absorb && plan.sp < 0;
            plan.success |= plan.sp != 0;
        }
        if !positive && !plan.success && (skill.affect_hp || skill.affect_sp) {
            return plan;
        }
        for (index, flag) in skill.affect_stats.into_iter().enumerate() {
            if flag && self.skill_roll(hit) {
                plan.stats[index] = self.stat_change(target, index, delta);
                plan.success |= plan.stats[index] != 0;
            }
        }
        self.plan_skill_states(target, skill, hit, &mut plan, &mut states);
        if plan.revived && amount > 0 {
            plan.hp = Some(if skill.affect_hp {
                amount
            } else {
                self.pools(target).0 * amount / 100
            });
        }
        plan
    }
}
