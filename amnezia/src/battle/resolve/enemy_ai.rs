use super::*;
use amnezia_data::EnemyActionDef;

impl Battle {
    pub(in crate::battle) fn enemy_action(&mut self, i: usize, alive: &[bool]) -> Option<Action> {
        if !self.enemies[i].alive() {
            return None;
        }
        self.enemies[i].switch_on_after_action = None;
        self.enemies[i].switch_off_after_action = None;
        let restriction = logic::worst_restriction(&self.enemies[i].states, &self.states);
        let kind = match restriction {
            1 => Command::Nothing,
            3 => {
                let others = self
                    .enemies
                    .iter()
                    .enumerate()
                    .map(|(j, enemy)| j != i && enemy.alive())
                    .collect::<Vec<_>>();
                logic::select_target(&others, rng_next(&mut self.rng) as usize)
                    .map_or(Command::Nothing, |target| Command::Attack { target })
            }
            2 => {
                let target = logic::select_target(alive, rng_next(&mut self.rng) as usize)?;
                Command::Attack { target }
            }
            _ => self.choose_enemy_command(i, alive),
        };
        Some(Action {
            source: Source::Enemy(i),
            kind,
            agility: self.enemies[i].stats.agility,
        })
    }

    fn choose_enemy_command(&mut self, i: usize, alive: &[bool]) -> Command {
        let context = self.enemy_ai_context(i);
        let roll = rng_next(&mut self.rng);
        let chosen = logic::choose_enemy_action(
            &self.enemies[i].actions,
            &context,
            |action| self.enemy_action_usable(i, action),
            |action| self.enemy_action_effective(i, action),
            |id| self.ai_switches.contains(&id),
            roll,
        );
        let Some(action) = chosen else {
            return Command::Nothing;
        };
        let (targets, random_target) = if action.kind == 1 {
            let skill = self
                .skills
                .iter()
                .find(|skill| skill.id == action.skill_id)
                .unwrap();
            match skill.scope {
                2 => (vec![i], false),
                3 | 4 => (self.enemy_ally_skill_targets(skill), skill.scope == 3),
                _ => (
                    alive
                        .iter()
                        .enumerate()
                        .filter_map(|(i, alive)| alive.then_some(i))
                        .collect(),
                    skill.scope == 0,
                ),
            }
        } else {
            (
                alive
                    .iter()
                    .enumerate()
                    .filter_map(|(i, alive)| alive.then_some(i))
                    .collect(),
                matches!(action.basic, 0 | 1),
            )
        };
        let roll = if random_target {
            rng_next(&mut self.rng) as usize
        } else {
            0
        };
        let target = targets
            .get(roll % targets.len().max(1))
            .copied()
            .unwrap_or(0);
        self.enemies[i].switch_on_after_action = action.switch_on.then_some(action.switch_on_id);
        self.enemies[i].switch_off_after_action = action.switch_off.then_some(action.switch_off_id);
        logic::enemy_command(Some(&action), target)
    }

    fn enemy_action_usable(&self, i: usize, action: &EnemyActionDef) -> bool {
        match action.kind {
            0 => action.basic <= 7,
            1 => self
                .skills
                .iter()
                .find(|skill| skill.id == action.skill_id)
                .is_some_and(|skill| self.skill_usable_by(Source::Enemy(i), skill)),
            _ => false,
        }
    }

    fn enemy_action_effective(&self, i: usize, action: &EnemyActionDef) -> bool {
        if action.kind != 1 {
            return true;
        }
        let Some(skill) = self.skills.iter().find(|skill| skill.id == action.skill_id) else {
            return false;
        };
        match skill.scope {
            2 => self.enemy_ally_skill_target(i, skill),
            3 | 4 => !self.enemy_ally_skill_targets(skill).is_empty(),
            _ => self.members.iter().any(|member| member.alive()),
        }
    }

    pub(in crate::battle::resolve) fn enemy_ally_skill_targets(
        &self,
        skill: &SkillDef,
    ) -> Vec<usize> {
        (0..self.enemies.len())
            .filter(|&i| self.enemy_ally_skill_target(i, skill))
            .collect()
    }

    pub(in crate::battle::resolve) fn enemy_ally_skill_target(
        &self,
        i: usize,
        skill: &SkillDef,
    ) -> bool {
        self.enemies.get(i).is_some_and(|enemy| {
            !enemy.fled
                && if !enemy.alive() {
                    skill.affected_states.contains(&1)
                } else {
                    skill.affect_hp
                        || skill.affect_sp
                        || skill
                            .affected_states
                            .iter()
                            .any(|&state| logic::has_state(&enemy.states, state))
                }
        })
    }

    pub(in crate::battle::resolve) fn enemy_ai_context(&self, i: usize) -> logic::EnemyAiContext {
        let enemy = &self.enemies[i];
        let level = self
            .members
            .iter()
            .map(|member| u64::from(member.level))
            .sum::<u64>()
            / self.members.len().max(1) as u64;
        let hp = self
            .members
            .iter()
            .map(|member| f64::from(member.hp.max(0)))
            .sum::<f64>();
        let max_hp = self
            .members
            .iter()
            .map(|member| f64::from(member.max_hp))
            .sum::<f64>()
            .max(1.0);
        let sp = self
            .members
            .iter()
            .map(|member| f64::from(member.sp.max(0)))
            .sum::<f64>();
        let max_sp = self
            .members
            .iter()
            .map(|member| f64::from(member.max_sp))
            .sum::<f64>()
            .max(1.0);
        let fatigue = if self.members.is_empty() {
            0
        } else {
            (100.0 - (100.0 * (2.0 * hp / max_hp + sp / max_sp) / 3.0).round()).clamp(0.0, 100.0)
                as u32
        };
        logic::EnemyAiContext {
            turn: self.events.turn,
            enemies: self.living_enemies().len() as u32,
            hp: logic::hp_percent(enemy.hp, enemy.max_hp),
            sp: logic::hp_percent(enemy.sp, enemy.max_sp),
            level: level as u32,
            fatigue,
        }
    }
}
