use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn log_basic_use(&mut self, action: Action) {
        let term = match action.kind {
            Command::Attack { .. } | Command::DoubleAttack { .. } => &self.text.attacking,
            Command::Defend => &self.text.defending,
            Command::Observe => &self.text.observing,
            Command::Charge => &self.text.focus,
            Command::SelfDestruct => &self.text.autodestruction,
            Command::Escape => &self.text.enemy_escape,
            _ => return,
        };
        self.log
            .push(format!("{}{term}", self.battler_name(action.source)));
    }

    pub(in crate::battle::resolve) fn log_damage_result(
        &mut self,
        target: Source,
        damage: Option<i32>,
    ) {
        let name = self.battler_name(target);
        let line = match damage {
            Some(amount) => self
                .text
                .damaged(name, matches!(target, Source::Party(_)), amount),
            None => self.text.skill_failed(name, 3),
        };
        self.log.push(line);
        if damage.is_some()
            && let Some(line) = self.death_message(target)
        {
            self.log.push(line);
        }
    }

    pub(in crate::battle::resolve) fn death_message(&self, target: Source) -> Option<String> {
        if self.battler_hp(target) > 0 {
            return None;
        }
        let state = self.states.iter().find(|state| state.id == 1)?;
        let suffix = match target {
            Source::Party(_) => &state.message_actor,
            Source::Enemy(_) => &state.message_enemy,
        };
        Some(format!("{}{suffix}", self.battler_name(target)))
    }
}
