use super::*;

impl Battle {
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
