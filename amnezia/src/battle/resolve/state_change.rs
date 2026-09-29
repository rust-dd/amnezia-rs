use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn state_restriction(&self, target: Source) -> u32 {
        logic::worst_restriction(self.battler_states(target), &self.states)
    }

    pub(in crate::battle::resolve) fn inflict_battler_state(
        &mut self,
        target: Source,
        id: u32,
    ) -> bool {
        let before = self.state_restriction(target);
        let active = match target {
            Source::Party(i) => &mut self.members[i].states,
            Source::Enemy(i) => &mut self.enemies[i].states,
        };
        if !logic::inflict_with_priority(active, id, &self.states) {
            return false;
        }
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
        self.states_changed(target, before, true);
        true
    }

    pub(in crate::battle::resolve) fn cure_battler_state(&mut self, target: Source, id: u32) {
        let before = self.state_restriction(target);
        logic::cure(self.battler_states_mut(target), id);
        self.states_changed(target, before, false);
    }

    pub(in crate::battle) fn mark_knocked_out(&mut self, target: Source) {
        self.clear_battler_bonuses(target);
        let active = match target {
            Source::Party(i) => &mut self.members[i].states,
            Source::Enemy(i) => &mut self.enemies[i].states,
        };
        if !logic::has_state(active, 1) {
            if self.states.iter().any(|state| state.id == 1) {
                logic::inflict_with_priority(active, 1, &self.states);
            } else {
                active.clear();
                logic::inflict(active, 1);
            }
        }
        self.cancel_pending_action(target);
    }

    pub(in crate::battle::resolve) fn states_changed(
        &mut self,
        target: Source,
        before: u32,
        inflicted: bool,
    ) {
        let restriction = self.state_restriction(target);
        if inflicted && restriction != 0 {
            match target {
                Source::Party(i) => self.members[i].defending = false,
                Source::Enemy(i) => {
                    self.enemies[i].defending = false;
                    self.enemies[i].charging = false;
                }
            }
        }
        let invalid_skill = self.queue[self.queue_at..]
            .iter()
            .filter(|action| action.source == target)
            .any(|action| match action.kind {
                Command::Skill { skill_id, .. } => !self
                    .skills
                    .iter()
                    .find(|skill| skill.id == skill_id)
                    .is_some_and(|skill| self.skill_usable_by(target, skill)),
                _ => false,
            });
        if before != restriction || (inflicted && restriction != 0) || invalid_skill {
            self.cancel_pending_action(target);
        }
    }

    fn cancel_pending_action(&mut self, target: Source) {
        let mut cancelled = false;
        for action in self
            .queue
            .iter_mut()
            .skip(self.queue_at)
            .filter(|action| action.source == target)
        {
            action.kind = Command::Nothing;
            cancelled = true;
        }
        match target {
            Source::Party(i) => {
                if self.members[i].command.is_some() {
                    self.members[i].command = Some(Command::Nothing);
                }
            }
            Source::Enemy(i) if cancelled => {
                self.enemies[i].switch_on_after_action = None;
                self.enemies[i].switch_off_after_action = None;
            }
            _ => {}
        }
    }
    pub(in crate::battle) fn clear_battler_bonuses(&mut self, target: Source) {
        match target {
            Source::Party(i) => {
                self.members[i].stat_modifiers = [0; 4];
                self.members[i].defending = false;
            }
            Source::Enemy(i) => {
                self.enemies[i].stat_modifiers = [0; 4];
                self.enemies[i].defending = false;
                self.enemies[i].charging = false;
            }
        }
    }
}
