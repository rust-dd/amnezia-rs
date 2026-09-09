use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn skill_stat_effects(
        &mut self,
        target: Source,
        skill: &SkillDef,
        hit: i32,
        amount: i32,
    ) -> Vec<String> {
        let delta = if skill.scope < 2 { -amount } else { amount };
        let mut lines = Vec::new();
        for (index, flag) in skill.affect_stats.into_iter().enumerate() {
            if !flag || !self.skill_roll(hit) {
                continue;
            }
            let base = self.battler_base_stats(target);
            let modifiers = match target {
                Source::Party(i) => &mut self.members[i].stat_modifiers,
                Source::Enemy(i) => &mut self.enemies[i].stat_modifiers,
            };
            let base = [base.attack, base.defense, base.spirit, base.agility][index] as i32;
            let old = modifiers[index];
            modifiers[index] = (old + delta).clamp(-(base / 2), base);
            let changed = modifiers[index] - old;
            if changed != 0 {
                lines.push(format!(
                    "{} {} {changed:+}",
                    self.battler_name(target),
                    ["ATK", "DEF", "SPI", "AGI"][index]
                ));
            }
        }
        lines
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
