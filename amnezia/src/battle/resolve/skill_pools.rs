use super::*;

impl Battle {
    pub(in crate::battle) fn skill_cost(&self, source: Source, skill: &SkillDef) -> u32 {
        self.battler_equipment_effects(source)
            .skill_cost(skill.sp_cost)
    }

    pub(in crate::battle) fn skill_usable_by(&self, source: Source, skill: &SkillDef) -> bool {
        let (alive, sp) = match source {
            Source::Party(i) => self.members.get(i).map(|f| (f.alive(), f.sp)),
            Source::Enemy(i) => self.enemies.get(i).map(|f| (f.alive(), f.sp)),
        }
        .unwrap_or_default();
        alive
            && skill.skill_type == 0
            && i64::from(sp) >= i64::from(self.skill_cost(source, skill))
            && logic::states_allow_skill(self.battler_states(source), &self.states, skill)
            && match source {
                Source::Party(i) => logic::weapon_allows_skill(
                    skill,
                    &self.members[i].weapon_attributes,
                    &self.attributes,
                ),
                Source::Enemy(_) => true,
            }
    }

    pub(in crate::battle::resolve) fn change_sp(&mut self, target: Source, amount: i32) -> i32 {
        let (sp, max) = match target {
            Source::Party(i) => {
                let fighter = &mut self.members[i];
                (&mut fighter.sp, fighter.max_sp)
            }
            Source::Enemy(i) => {
                let enemy = &mut self.enemies[i];
                (&mut enemy.sp, enemy.max_sp)
            }
        };
        let before = *sp;
        *sp = sp.saturating_add(amount).clamp(0, max);
        *sp - before
    }
}
