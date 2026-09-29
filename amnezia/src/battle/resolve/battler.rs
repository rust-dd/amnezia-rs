use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn source_alive(&self, source: Source) -> bool {
        match source {
            Source::Party(i) => self.members.get(i).is_some_and(|f| f.alive()),
            Source::Enemy(i) => self.enemies.get(i).is_some_and(|e| e.alive()),
        }
    }

    pub(in crate::battle::resolve) fn battler_attribute_damage(
        &self,
        base: i32,
        target: Source,
        attributes: &[u32],
    ) -> i32 {
        logic::attribute_damage(base, attributes, &self.attributes, |id| {
            let Some(index) = id.checked_sub(1) else {
                return 2;
            };
            match target {
                Source::Party(i) => {
                    let member = &self.members[i];
                    let rank = member
                        .attribute_ranks
                        .get(index as usize)
                        .copied()
                        .unwrap_or(2);
                    rank.saturating_add(u8::from(member.resist_attributes.contains(&id)))
                        .min(4)
                }
                Source::Enemy(i) => self.enemies[i]
                    .attribute_ranks
                    .get(index as usize)
                    .copied()
                    .unwrap_or(2),
            }
        })
    }

    pub(in crate::battle) fn battler_stats(&self, target: Source) -> logic::Stats {
        let stats = self.battler_base_stats(target);
        let modifiers = match target {
            Source::Party(i) => self.members[i].stat_modifiers,
            Source::Enemy(i) => self.enemies[i].stat_modifiers,
        };
        let values = [stats.attack, stats.defense, stats.spirit, stats.agility];
        let [attack, defense, spirit, agility] =
            std::array::from_fn(|i| (values[i] as i32 + modifiers[i]).clamp(1, 9999) as u32);
        logic::state_stats(
            logic::Stats {
                attack,
                defense,
                spirit,
                agility,
            },
            self.battler_states(target),
            &self.states,
        )
    }

    pub(in crate::battle::resolve) fn battler_base_stats(&self, target: Source) -> logic::Stats {
        match target {
            Source::Party(i) => {
                let stats = self.members[i].stats;
                logic::Stats {
                    attack: stats.attack.clamp(1, 999),
                    defense: stats.defense.clamp(1, 999),
                    spirit: stats.spirit.clamp(1, 999),
                    agility: stats.agility.clamp(1, 999),
                }
            }
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

    pub(in crate::battle) fn battler_states(&self, target: Source) -> &[(u32, u32)] {
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
        let hit = logic::skill_to_hit(
            skill,
            self.battler_stats(source).agility,
            self.battler_stats(target).agility,
            can_act,
            logic::state_hit_ratio(self.battler_states(source), &self.states),
        );
        if skill.failure_message == 3 && skill.scope < 2 {
            self.battler_equipment_effects(target)
                .physical_hit(hit, can_act)
        } else {
            hit
        }
    }
}
