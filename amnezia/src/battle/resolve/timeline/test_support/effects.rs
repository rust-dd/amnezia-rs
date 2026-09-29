use super::*;

pub(in crate::battle::resolve) struct StateEffects {
    pub lines: Vec<String>,
    pub success: bool,
}

impl Battle {
    pub(in crate::battle::resolve) fn release_states_from_damage(
        &mut self,
        target: Source,
        rate: u32,
    ) {
        if self.battler_hp(target) <= 0 || rate == 0 {
            return;
        }
        let mut plan = TargetPlan {
            success: true,
            ..Default::default()
        };
        let mut states = self.planning_states(target);
        self.plan_releases(&mut plan, &mut states, rate);
        self.run_effect(target, target, plan, Stage::Released(0));
    }

    pub(in crate::battle::resolve) fn weapon_states(&mut self, source: Source, target: Source) {
        if self.battler_hp(target) <= 0 {
            return;
        }
        let mut plan = TargetPlan {
            success: true,
            ..Default::default()
        };
        let mut states = self.planning_states(target);
        self.plan_weapon_states(source, target, &mut plan, &mut states);
        self.run_effect(source, target, plan, Stage::State(0));
    }

    pub(in crate::battle::resolve) fn skill_states(
        &mut self,
        target: Source,
        skill: &SkillDef,
        hit: i32,
    ) -> StateEffects {
        let mut plan = TargetPlan {
            positive: matches!(skill.scope, 2..=4),
            ..Default::default()
        };
        let mut states = self.planning_states(target);
        self.plan_skill_states(target, skill, hit, &mut plan, &mut states);
        StateEffects {
            success: plan.success,
            lines: self.run_effect(target, target, plan, Stage::State(0)),
        }
    }

    pub(in crate::battle::resolve) fn skill_stat_effects(
        &mut self,
        target: Source,
        skill: &SkillDef,
        hit: i32,
        amount: i32,
    ) -> Vec<String> {
        let mut plan = TargetPlan {
            success: true,
            ..Default::default()
        };
        let amount = if skill.scope < 2 { -amount } else { amount };
        for (i, flag) in skill.affect_stats.into_iter().enumerate() {
            if flag && self.skill_roll(hit) {
                plan.stats[i] = self.stat_change(target, i, amount);
            }
        }
        self.run_effect(target, target, plan, Stage::Parameter(0))
    }
}
