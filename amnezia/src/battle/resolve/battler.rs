use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn battler_stats(&self, target: Source) -> logic::Stats {
        match target {
            Source::Party(i) => self.members[i].stats,
            Source::Enemy(i) => self.enemies[i].stats,
        }
    }

    pub(in crate::battle::resolve) fn battler_name(&self, target: Source) -> &str {
        match target {
            Source::Party(i) => &self.members[i].name,
            Source::Enemy(i) => &self.enemies[i].name,
        }
    }

    pub(in crate::battle::resolve) fn battler_hp(&self, target: Source) -> i32 {
        match target {
            Source::Party(i) => self.members[i].hp,
            Source::Enemy(i) => self.enemies[i].hp,
        }
    }

    pub(in crate::battle::resolve) fn battler_pos(&self, target: Source) -> (f32, f32) {
        match target {
            Source::Party(i) => (self.party_anim_x(i), PARTY_ANIM_Y),
            Source::Enemy(i) => self.foe_anim_pos(i),
        }
    }

    pub(in crate::battle::resolve) fn battler_states(&self, target: Source) -> &[(u32, u32)] {
        match target {
            Source::Party(i) => &self.members[i].states,
            Source::Enemy(i) => &self.enemies[i].states,
        }
    }

    pub(in crate::battle::resolve) fn battler_states_mut(
        &mut self,
        target: Source,
    ) -> &mut Vec<(u32, u32)> {
        match target {
            Source::Party(i) => &mut self.members[i].states,
            Source::Enemy(i) => &mut self.enemies[i].states,
        }
    }

    pub(in crate::battle::resolve) fn battler_state_probability(
        &self,
        target: Source,
        id: u32,
    ) -> u32 {
        let Some(index) = id.checked_sub(1) else {
            return 0;
        };
        let Some(state) = self.states.iter().find(|state| state.id == id) else {
            return 0;
        };
        let (rank, multiplier) = match target {
            Source::Party(i) => (
                self.members[i]
                    .state_ranks
                    .get(index as usize)
                    .copied()
                    .unwrap_or(2),
                self.members[i]
                    .state_guards
                    .iter()
                    .find(|(state, _)| *state == id)
                    .map_or(100, |(_, multiplier)| *multiplier),
            ),
            Source::Enemy(i) => (
                self.enemies[i]
                    .state_ranks
                    .get(index as usize)
                    .copied()
                    .unwrap_or(1),
                100,
            ),
        };
        logic::state_infliction_chance(state, rank) * multiplier / 100
    }

    pub(in crate::battle::resolve) fn skill_roll(&mut self, chance: i32) -> bool {
        ((rng_next(&mut self.rng) % 100) as i32) < chance
    }

    pub(in crate::battle::resolve) fn skill_hit_chance(
        &self,
        source: Source,
        target: Source,
        skill: &SkillDef,
    ) -> i32 {
        let can_act = logic::worst_restriction(self.battler_states(target), &self.states) != 1;
        logic::skill_to_hit(
            skill,
            self.battler_stats(source).agility,
            self.battler_stats(target).agility,
            can_act,
        )
    }
}
