//! The enemy side of a round: picking each foe's AI action, driving the party
//! command flow past restricted members (and RM2000 Auto-battle), and retargeting
//! a chosen target that has since fallen.

use super::*;

impl Battle {
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
            let forced = self.forced_party_command(restriction);
            self.members[i].command = Some(forced);
            self.members[i].defending = false;
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
                self.forced_party_command(restriction)
            };
            self.members[i].command = Some(command);
            self.members[i].defending = false;
        }
        self.begin_resolve();
    }

    /// Berserk targets enemies; confusion can hit any ally, including oneself.
    fn forced_party_command(&mut self, restriction: u32) -> Command {
        let alive = match restriction {
            2 => self.enemies.iter().map(|e| e.alive()).collect::<Vec<_>>(),
            3 => self.members.iter().map(|m| m.alive()).collect::<Vec<_>>(),
            _ => return Command::Nothing,
        };
        match logic::select_target(&alive, rng_next(&mut self.rng) as usize) {
            Some(target) => Command::Attack { target },
            None => Command::Nothing,
        }
    }
}
