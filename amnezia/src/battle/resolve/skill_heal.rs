use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn skill_heal_battler(
        &mut self,
        source: Source,
        target: Source,
        skill: &SkillDef,
    ) -> Vec<String> {
        let (caster_name, caster_stats) = match source {
            Source::Party(i) => (self.members[i].name.clone(), self.members[i].stats),
            Source::Enemy(i) => (self.enemies[i].name.clone(), self.enemies[i].stats),
        };
        let (target_name, target_stats, pos) = match target {
            Source::Party(i) => (
                self.members[i].name.clone(),
                self.members[i].stats,
                (self.party_anim_x(i), PARTY_ANIM_Y),
            ),
            Source::Enemy(i) => (
                self.enemies[i].name.clone(),
                self.enemies[i].stats,
                self.foe_anim_pos(i),
            ),
        };
        let base = logic::skill_effect(skill, &caster_stats, &target_stats, false);
        let amount =
            logic::variance_adjust(base, skill.variance as i32, rng_next(&mut self.rng)).max(0);
        let (hp, sp, max_hp, max_sp, states) = match target {
            Source::Party(i) => {
                let member = &mut self.members[i];
                (
                    &mut member.hp,
                    &mut member.sp,
                    member.max_hp,
                    member.max_sp,
                    &mut member.states,
                )
            }
            Source::Enemy(i) => {
                let enemy = &mut self.enemies[i];
                (
                    &mut enemy.hp,
                    &mut enemy.sp,
                    enemy.max_hp,
                    enemy.max_sp,
                    &mut enemy.states,
                )
            }
        };
        let old_hp = (*hp).max(0);
        let old_sp = *sp;
        let was_dead = old_hp == 0;
        if skill.affect_sp {
            *sp = (*sp + amount).min(max_sp);
        }
        let hp_gain = if skill.affect_hp {
            amount
        } else if was_dead {
            // Without the HP flag, RPG_RT treats revival power as a percentage.
            max_hp * amount / 100
        } else {
            0
        };
        *hp = (old_hp + hp_gain).min(max_hp);
        if was_dead {
            *hp = (*hp).max(1);
        }
        let shown = (*hp - old_hp).max(*sp - old_sp);
        let mut lines = vec![format!("{caster_name} varázsol: {target_name} +{shown}")];
        for &id in &skill.affected_states {
            if logic::has_state(states, id) {
                logic::cure(states, id);
                if let Some(state) = self.states.iter().find(|state| state.id == id) {
                    lines.push(format!("{target_name} gyógyul: {}", state.name));
                }
            }
        }
        if let Source::Enemy(i) = target
            && was_dead
        {
            self.enemies[i].dying = None;
        }
        if shown > 0 {
            self.push_number(pos, shown.to_string(), NumberKind::Heal);
        }
        lines
    }
}
