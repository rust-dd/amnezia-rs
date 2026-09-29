use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn skill_magnitude(
        &mut self,
        source: Source,
        target: Source,
        skill: &SkillDef,
    ) -> i32 {
        let base = logic::skill_effect(
            skill,
            &self.battler_stats(source),
            &self.battler_stats(target),
            skill.scope < 2,
        );
        let base = self.battler_attribute_damage(base, target, &skill.attributes);
        if skill.variance == 0 || base <= 0 {
            base
        } else {
            logic::variance_adjust(base, skill.variance as i32, rng_next(&mut self.rng)).max(0)
        }
    }

    pub(in crate::battle::resolve) fn restore_skill_hp(
        &mut self,
        target: Source,
        amount: i32,
    ) -> i32 {
        let (hp, max) = match target {
            Source::Party(i) => {
                let member = &mut self.members[i];
                (&mut member.hp, member.max_hp)
            }
            Source::Enemy(i) => {
                let enemy = &mut self.enemies[i];
                (&mut enemy.hp, enemy.max_hp)
            }
        };
        let before = *hp;
        *hp = hp.saturating_add(amount).clamp(0, max);
        *hp - before
    }
}
