use super::*;

#[derive(Default)]
pub(in crate::battle::resolve) struct StateEffects {
    pub lines: Vec<String>,
    pub success: bool,
    pub revived: bool,
}

impl Battle {
    pub(in crate::battle::resolve) fn skill_states(
        &mut self,
        target: Source,
        skill: &SkillDef,
        hit: i32,
    ) -> StateEffects {
        let heals = matches!(skill.scope, 2..=4);
        let mut effects = StateEffects::default();
        for &id in &skill.affected_states {
            let present = if id == 1 {
                self.battler_hp(target) <= 0
            } else {
                logic::has_state(self.battler_states(target), id)
            };
            if present && !heals {
                effects.success = true;
                if let Some(state) = self.states.iter().find(|state| state.id == id) {
                    effects.lines.push(format!(
                        "{}{}",
                        self.battler_name(target),
                        state.message_already
                    ));
                }
                continue;
            }
            if !self.skill_roll(hit) {
                continue;
            }
            if heals {
                if !present {
                    continue;
                }
                self.cure_battler_state(target, id);
                effects.revived |= id == 1;
            } else {
                let chance = self.battler_state_probability(target, id);
                if !self.skill_roll(chance as i32) {
                    continue;
                }
                if !self.inflict_battler_state(target, id) {
                    continue;
                }
            }
            effects.success = true;
            if let Some(state) = self.states.iter().find(|state| state.id == id) {
                let message = if heals {
                    &state.message_recovery
                } else if matches!(target, Source::Party(_)) {
                    &state.message_actor
                } else {
                    &state.message_enemy
                };
                effects
                    .lines
                    .push(format!("{}{message}", self.battler_name(target)));
            }
        }
        effects
    }
}
