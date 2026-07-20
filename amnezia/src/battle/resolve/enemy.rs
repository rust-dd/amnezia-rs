//! The enemy side of a round: picking each foe's AI action, driving the party
//! command flow past restricted members (and RM2000 Auto-battle), and retargeting
//! a chosen target that has since fallen.

use super::*;

impl Battle {
    /// Choose living enemy `i`'s action for the coming round. A status restriction
    /// overrides the AI: can't-act does nothing, berserk forces a plain attack on a
    /// party member, confusion an attack on a random other foe (friendly fire).
    /// Otherwise a random living party target, this foe's and the party's HP
    /// percentages, and the round number feed [`logic::choose_enemy_action`], whose
    /// result [`logic::enemy_command`] maps to a [`Command`] (a basic attack when
    /// nothing is eligible). `None` when the enemy is down or no member is left to
    /// target.
    pub(in crate::battle) fn enemy_action(&mut self, i: usize, alive: &[bool]) -> Option<Action> {
        if !self.enemies[i].alive() {
            return None;
        }
        let agility = self.enemies[i].stats.agility;
        let restriction = logic::worst_restriction(&self.enemies[i].states, &self.states);
        let kind = match restriction {
            1 => Command::Nothing,
            3 => {
                // Confusion turns the blow on a random other living foe (resolved
                // as friendly fire in `apply`); none left -> nothing.
                let others: Vec<bool> = self
                    .enemies
                    .iter()
                    .enumerate()
                    .map(|(j, e)| j != i && e.alive())
                    .collect();
                match logic::select_target(&others, rng_next(&mut self.rng) as usize) {
                    Some(target) => Command::Attack { target },
                    None => Command::Nothing,
                }
            }
            2 => {
                // Berserk forces a plain attack on a random living party member.
                let target = logic::select_target(alive, rng_next(&mut self.rng) as usize)?;
                Command::Attack { target }
            }
            _ => {
                let target = logic::select_target(alive, rng_next(&mut self.rng) as usize)?;
                let party_hp: i32 = self.members.iter().map(|f| f.hp.max(0)).sum();
                let party_max: i32 = self.members.iter().map(|f| f.max_hp).sum();
                let enemy_hp_pct = logic::hp_percent(self.enemies[i].hp, self.enemies[i].max_hp);
                let party_hp_pct = logic::hp_percent(party_hp, party_max);
                let chosen = logic::choose_enemy_action(
                    &self.enemies[i].actions,
                    enemy_hp_pct,
                    party_hp_pct,
                    logic::AI_PARTY_LEVEL,
                    self.round,
                    rng_next(&mut self.rng),
                );
                logic::enemy_command(chosen.as_ref(), target)
            }
        };
        Some(Action {
            source: Source::Enemy(i),
            kind,
            agility,
        })
    }

    /// Advance the command phase past every member the game must act for: a
    /// can't-act (restriction 1) member is auto-ordered [`Command::Nothing`]; a
    /// berserk (2) member is forced to strike a random living enemy; a confused (3)
    /// member a random living ally. Stops on the first member who may freely choose
    /// (setting [`Battle::turn`]), or enters resolution once every remaining chooser
    /// has been auto-ordered — so the command UI never halts on a restricted member.
    pub(in crate::battle) fn skip_restricted_choosers(&mut self) {
        while let Some(i) = self.next_chooser() {
            let restriction = logic::worst_restriction(&self.members[i].states, &self.states);
            if restriction == 0 {
                self.turn = i;
                return;
            }
            let forced = self.forced_party_command(i, restriction);
            self.members[i].command = Some(forced);
        }
        self.begin_resolve();
    }

    /// Order every living member for RM2000 Auto-battle, then resolve: a freely
    /// acting member basic-attacks a random living enemy (nothing when no foe is
    /// left), while a restricted member keeps its forced action. Reuses the same
    /// target/AI helpers as the per-actor flow, so the enemies still act.
    pub(in crate::battle) fn auto_battle_commands(&mut self) {
        let enemies_alive: Vec<bool> = self.enemies.iter().map(|e| e.alive()).collect();
        for i in 0..self.members.len() {
            if !self.members[i].alive() {
                continue;
            }
            let restriction = logic::worst_restriction(&self.members[i].states, &self.states);
            let command = if restriction == 0 {
                match logic::select_target(&enemies_alive, rng_next(&mut self.rng) as usize) {
                    Some(target) => Command::Attack { target },
                    None => Command::Nothing,
                }
            } else {
                self.forced_party_command(i, restriction)
            };
            self.members[i].command = Some(command);
        }
        self.begin_resolve();
    }

    /// The attack a restricted member `i` is forced into: a random living enemy
    /// while berserk (2), a random living ally while confused (3), or nothing at all
    /// (restriction 1, or no legal target). The chosen side is re-checked and the
    /// blow re-aimed at resolution by [`Battle::apply`].
    fn forced_party_command(&mut self, i: usize, restriction: u32) -> Command {
        let alive: Vec<bool> = match restriction {
            2 => self.enemies.iter().map(|e| e.alive()).collect(),
            3 => self
                .members
                .iter()
                .enumerate()
                .map(|(j, m)| j != i && m.alive())
                .collect(),
            _ => return Command::Nothing,
        };
        match logic::select_target(&alive, rng_next(&mut self.rng) as usize) {
            Some(target) => Command::Attack { target },
            None => Command::Nothing,
        }
    }

    /// Keep `target` if that enemy still lives, else pick another living enemy.
    pub(in crate::battle::resolve) fn retarget_enemy(&mut self, target: usize) -> Option<usize> {
        if self.enemies.get(target).is_some_and(|e| e.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self.enemies.iter().map(|e| e.alive()).collect();
        let roll = rng_next(&mut self.rng) as usize;
        logic::select_target(&alive, roll)
    }

    pub(in crate::battle::resolve) fn retarget_member(&mut self, target: usize) -> Option<usize> {
        if self.members.get(target).is_some_and(|m| m.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self.members.iter().map(|m| m.alive()).collect();
        let roll = rng_next(&mut self.rng) as usize;
        logic::select_target(&alive, roll)
    }

    /// Keep confused member `pi`'s stored ally `target` if it still lives, else pick
    /// another living ally. `None` when `pi` has no living ally to turn on.
    pub(in crate::battle::resolve) fn retarget_ally(
        &mut self,
        pi: usize,
        target: usize,
    ) -> Option<usize> {
        if target != pi && self.members.get(target).is_some_and(|m| m.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self
            .members
            .iter()
            .enumerate()
            .map(|(j, m)| j != pi && m.alive())
            .collect();
        logic::select_target(&alive, rng_next(&mut self.rng) as usize)
    }

    /// Keep confused foe `ei`'s stored fellow `target` if it still lives, else pick
    /// another living foe. `None` when `ei` has no living fellow to turn on.
    pub(in crate::battle::resolve) fn retarget_other_enemy(
        &mut self,
        ei: usize,
        target: usize,
    ) -> Option<usize> {
        if target != ei && self.enemies.get(target).is_some_and(|e| e.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self
            .enemies
            .iter()
            .enumerate()
            .map(|(j, e)| j != ei && e.alive())
            .collect();
        logic::select_target(&alive, rng_next(&mut self.rng) as usize)
    }
}
