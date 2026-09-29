use super::*;

mod item;
mod physical;
mod skill;
mod states;

#[derive(Default)]
pub(in crate::battle::resolve) struct TargetPlan {
    pub success: bool,
    pub critical: bool,
    pub hp: Option<i32>,
    pub sp: i32,
    pub stats: [i32; 4],
    pub states: Vec<StateChange>,
    pub positive: bool,
    pub absorb_hp: bool,
    pub absorb_sp: bool,
    pub revived: bool,
    pub failure: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::battle::resolve) enum StateEffect {
    Released,
    Healed,
    Inflicted,
    Already,
}

#[derive(Clone, Copy)]
pub(in crate::battle::resolve) struct StateChange {
    pub id: u32,
    pub effect: StateEffect,
}

impl TargetPlan {
    pub fn damages_hp(&self) -> bool {
        self.hp
            .is_some_and(|hp| hp < 0 || hp == 0 && !self.positive && !self.absorb_hp)
    }
}

impl Battle {
    pub(in crate::battle::resolve) fn defending(&self, target: Source) -> bool {
        match target {
            Source::Party(i) => self.members[i].defending,
            Source::Enemy(i) => self.enemies[i].defending,
        }
    }

    pub(in crate::battle::resolve) fn pools(&self, target: Source) -> (i32, i32, i32) {
        match target {
            Source::Party(i) => {
                let member = &self.members[i];
                (member.max_hp, member.sp, member.max_sp)
            }
            Source::Enemy(i) => {
                let enemy = &self.enemies[i];
                (enemy.max_hp, enemy.sp, enemy.max_sp)
            }
        }
    }

    pub(in crate::battle::resolve) fn stat_change(
        &self,
        target: Source,
        index: usize,
        amount: i32,
    ) -> i32 {
        let base = self.battler_base_stats(target);
        let base = [base.attack, base.defense, base.spirit, base.agility][index] as i32;
        let old = match target {
            Source::Party(i) => self.members[i].stat_modifiers[index],
            Source::Enemy(i) => self.enemies[i].stat_modifiers[index],
        };
        (old + amount).clamp(-(base / 2), base) - old
    }

    pub(in crate::battle::resolve) fn variance(&mut self, base: i32, amount: u32) -> i32 {
        if base > 0 && amount > 0 {
            logic::variance_adjust(base, amount as i32, rng_next(&mut self.rng))
        } else {
            base
        }
    }
}
