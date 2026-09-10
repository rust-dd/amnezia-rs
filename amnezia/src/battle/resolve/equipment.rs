use super::*;
use crate::equipment::EquipmentEffects;

impl Battle {
    pub(in crate::battle::resolve) fn battler_equipment_effects(
        &self,
        source: Source,
    ) -> EquipmentEffects {
        match source {
            Source::Party(i) => self
                .members
                .get(i)
                .map(|f| f.equipment_effects)
                .unwrap_or_default(),
            Source::Enemy(_) => EquipmentEffects::default(),
        }
    }

    pub(in crate::battle::resolve) fn weapon_states(&mut self, source: Source, target: Source) {
        if self.battler_hp(target) <= 0 {
            return;
        }
        let Source::Party(i) = source else { return };
        for (id, chance) in self.members[i].weapon_states.clone() {
            let chance = chance * self.battler_state_probability(target, id) / 100;
            if self.skill_roll(chance as i32)
                && !logic::has_state(self.battler_states(target), id)
                && self.inflict_battler_state(target, id)
            {
                if id == 1 {
                    // The damage result owns the single collapse message.
                    continue;
                }
                let state = self.states.iter().find(|state| state.id == id).unwrap();
                let message = match target {
                    Source::Party(_) => &state.message_actor,
                    Source::Enemy(_) => &state.message_enemy,
                };
                self.log
                    .push(format!("{}{message}", self.battler_name(target)));
            }
        }
    }
}
