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
                continue;
            }
            if !self.skill_roll(hit) {
                continue;
            }
            if heals {
                if !present {
                    continue;
                }
                logic::cure(self.battler_states_mut(target), id);
                effects.revived |= id == 1;
            } else {
                let chance = self.battler_state_probability(target, id);
                if !self.skill_roll(chance as i32) {
                    continue;
                }
                logic::inflict(self.battler_states_mut(target), id);
                if id == 1 {
                    self.clear_battler_bonuses(target);
                    match target {
                        Source::Party(i) => self.members[i].hp = 0,
                        Source::Enemy(i) => {
                            self.enemies[i].hp = 0;
                            self.start_foe_death(i, false);
                        }
                    }
                }
            }
            effects.success = true;
            if let Some(state) = self.states.iter().find(|state| state.id == id) {
                let verb = if heals { "gyógyul" } else { "státusz" };
                effects.lines.push(format!(
                    "{} {verb}: {}",
                    self.battler_name(target),
                    state.name
                ));
            }
        }
        effects
    }
}
