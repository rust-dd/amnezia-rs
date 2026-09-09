use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn skill_heal_battler(
        &mut self,
        source: Source,
        target: Source,
        skill: &SkillDef,
    ) -> Vec<String> {
        let caster_name = self.battler_name(source).to_string();
        let target_name = self.battler_name(target).to_string();
        let amount = self.skill_magnitude(source, target, skill);
        let hit = self.skill_hit_chance(source, target, skill);
        let hp_hit = skill.affect_hp && self.skill_roll(hit);
        let sp_hit = skill.affect_sp && self.skill_roll(hit);
        let stats = self.skill_stat_effects(target, skill, hit, amount);
        let effects = self.skill_states(target, skill, hit);
        let (hp, sp, max_hp, max_sp) = match target {
            Source::Party(i) => {
                let member = &mut self.members[i];
                (&mut member.hp, &mut member.sp, member.max_hp, member.max_sp)
            }
            Source::Enemy(i) => {
                let enemy = &mut self.enemies[i];
                (&mut enemy.hp, &mut enemy.sp, enemy.max_hp, enemy.max_sp)
            }
        };
        let old_hp = (*hp).max(0);
        let old_sp = *sp;
        let was_dead = old_hp == 0;
        if sp_hit {
            *sp = (*sp + amount).min(max_sp);
        }
        let hp_gain = if effects.revived {
            // Without the HP flag, RPG_RT treats revival power as a percentage.
            if skill.affect_hp {
                amount
            } else {
                max_hp * amount / 100
            }
        } else if hp_hit && !was_dead {
            amount
        } else {
            0
        };
        *hp = (old_hp + hp_gain).min(max_hp);
        if effects.revived {
            *hp = (*hp).max(1);
        }
        let shown = (*hp - old_hp).max(*sp - old_sp);
        let success = hp_hit && amount > 0 && !was_dead
            || *sp != old_sp
            || effects.success
            || !stats.is_empty();
        if let Source::Enemy(i) = target
            && effects.revived
        {
            self.enemies[i].dying = None;
        }
        if shown > 0 {
            self.push_number(
                self.battler_pos(target),
                shown.to_string(),
                NumberKind::Heal,
            );
        }
        let mut lines = if success {
            vec![format!("{caster_name} varázsol: {target_name} +{shown}")]
        } else {
            vec![format!("{caster_name} varázsol: {target_name} hatástalan")]
        };
        lines.extend(stats);
        lines.extend(effects.lines);
        lines
    }
}
