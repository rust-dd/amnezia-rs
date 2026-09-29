use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn planning_states(&self, target: Source) -> Vec<(u32, u32)> {
        let mut states = self.battler_states(target).to_vec();
        if self.battler_hp(target) <= 0 && !logic::has_state(&states, 1) {
            states.push((1, 0));
        }
        states.sort_unstable_by_key(|(id, _)| *id);
        states
    }

    pub(in crate::battle::resolve) fn plan_releases(
        &mut self,
        plan: &mut TargetPlan,
        states: &mut Vec<(u32, u32)>,
        rate: u32,
    ) {
        let ids = logic::release_on_damage(states, &self.states, rate, || {
            (rng_next(&mut self.rng) % 100) as u32
        });
        plan.states.extend(ids.into_iter().map(|id| StateChange {
            id,
            effect: StateEffect::Released,
        }));
    }

    pub(in crate::battle::resolve) fn plan_weapon_states(
        &mut self,
        source: Source,
        target: Source,
        plan: &mut TargetPlan,
        states: &mut Vec<(u32, u32)>,
    ) {
        let Source::Party(i) = source else { return };
        let mut effects = self.members[i].weapon_states.clone();
        effects.sort_unstable_by_key(|(id, _)| *id);
        for (id, chance) in effects {
            let enabled = chance > 0;
            let chance = chance * self.battler_state_probability(target, id) / 100;
            if enabled
                && self.skill_roll(chance as i32)
                && !logic::has_state(states, id)
                && logic::inflict_with_priority(states, id, &self.states)
            {
                plan.states.push(StateChange {
                    id,
                    effect: StateEffect::Inflicted,
                });
            }
        }
    }

    pub(in crate::battle::resolve) fn plan_skill_states(
        &mut self,
        target: Source,
        skill: &SkillDef,
        hit: i32,
        plan: &mut TargetPlan,
        states: &mut Vec<(u32, u32)>,
    ) {
        for &id in &skill.affected_states {
            let present = logic::has_state(states, id);
            if present && !plan.positive {
                plan.success = true;
                plan.states.push(StateChange {
                    id,
                    effect: StateEffect::Already,
                });
                continue;
            }
            if !self.skill_roll(hit) {
                continue;
            }
            let effect = if plan.positive {
                if !present {
                    continue;
                }
                logic::cure(states, id);
                plan.revived |= id == 1;
                StateEffect::Healed
            } else {
                let chance = self.battler_state_probability(target, id);
                if !self.skill_roll(chance as i32)
                    || !logic::inflict_with_priority(states, id, &self.states)
                {
                    continue;
                }
                StateEffect::Inflicted
            };
            plan.success = true;
            plan.states.push(StateChange { id, effect });
        }
    }
}
