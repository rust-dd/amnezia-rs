use super::*;

impl Battle {
    /// Apply remaining states after recovery. Battle poison is non-lethal.
    pub(in crate::battle::resolve) fn tick_state_hp(&mut self, source: Source) {
        if self.battler_hp(source) <= 0 {
            return;
        }
        let (max_hp, max_sp) = match source {
            Source::Party(i) => (self.members[i].max_hp, self.members[i].max_sp),
            Source::Enemy(i) => (self.enemies[i].max_hp, self.enemies[i].max_sp),
        };
        let changes = self
            .states
            .iter()
            .filter(|state| logic::has_state(self.battler_states(source), state.id))
            .map(|state| {
                (
                    logic::state_hp_delta(state, max_hp),
                    logic::state_sp_delta(state, max_sp),
                )
            })
            .collect::<Vec<_>>();
        for (hp_delta, sp_delta) in changes {
            let (hp, sp) = match source {
                Source::Party(i) => {
                    let member = &mut self.members[i];
                    (&mut member.hp, &mut member.sp)
                }
                Source::Enemy(i) => {
                    let enemy = &mut self.enemies[i];
                    (&mut enemy.hp, &mut enemy.sp)
                }
            };
            *hp = hp.saturating_add(hp_delta).clamp(1, max_hp);
            *sp = sp.saturating_add(sp_delta).clamp(0, max_sp);
        }
    }

    pub(in crate::battle::resolve) fn recover_before_action(&mut self, target: Source) {
        let before = self.state_restriction(target);
        let active = match target {
            Source::Party(i) => &mut self.members[i].states,
            Source::Enemy(i) => &mut self.enemies[i].states,
        };
        let lifted = logic::tick_recovery(active, &self.states, || {
            (rng_next(&mut self.rng) % 100) as u32
        });
        self.log_action_state(target, &lifted);
        self.states_changed(target, before, false);
    }

    fn log_action_state(&mut self, target: Source, lifted: &[u32]) {
        let Some(state) = self
            .states
            .iter()
            .filter(|state| {
                lifted.contains(&state.id)
                    || logic::has_state(self.battler_states(target), state.id)
            })
            .max_by_key(|state| (state.priority, state.id))
        else {
            return;
        };
        let healed = lifted.contains(&state.id);
        let message = if healed {
            &state.message_recovery
        } else {
            &state.message_affected
        };
        if healed || !message.is_empty() {
            self.log
                .push(format!("{}{message}", self.battler_name(target)));
        }
    }

    pub(in crate::battle::resolve) fn release_states_from_damage(
        &mut self,
        target: Source,
        physical_rate: u32,
    ) {
        if self.battler_hp(target) <= 0 || physical_rate == 0 {
            return;
        }
        let before = self.state_restriction(target);
        let active = match target {
            Source::Party(i) => &mut self.members[i].states,
            Source::Enemy(i) => &mut self.enemies[i].states,
        };
        let lifted = logic::release_on_damage(active, &self.states, physical_rate, || {
            (rng_next(&mut self.rng) % 100) as u32
        });
        self.log_state_recovery(target, &lifted);
        self.states_changed(target, before, false);
    }

    fn log_state_recovery(&mut self, target: Source, lifted: &[u32]) {
        for id in lifted {
            if let Some(state) = self.states.iter().find(|state| state.id == *id) {
                self.log.push(format!(
                    "{}{}",
                    self.battler_name(target),
                    state.message_recovery
                ));
            }
        }
    }
}
