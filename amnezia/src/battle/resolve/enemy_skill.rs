use super::*;

impl Battle {
    /// Execute an enemy skill after checking its SP cost.
    pub(in crate::battle::resolve) fn enemy_cast(
        &mut self,
        ei: usize,
        skill_id: u32,
        target: usize,
    ) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id).cloned()?;
        let name = self.enemies[ei].name.clone();
        if !self.suppress_anim {
            if !self.pay_skill(Source::Enemy(ei), &skill) {
                return None;
            }
            let anchors = self.enemy_skill_anim_anchors(ei, &skill, target);
            self.push_anim(skill.animation_id, anchors);
        }
        if matches!(skill.scope, 2..=4) {
            let base = logic::skill_effect(
                &skill,
                &self.enemies[ei].stats,
                &self.enemies[ei].stats,
                false,
            );
            let roll = rng_next(&mut self.rng);
            let amt = logic::variance_adjust(base, skill.variance as i32, roll).max(0);
            let e = &mut self.enemies[ei];
            let old_hp = e.hp;
            let old_sp = e.sp;
            if skill.affect_hp {
                e.hp = (e.hp + amt).min(e.max_hp);
            }
            if skill.affect_sp {
                e.sp = (e.sp + amt).min(e.max_sp);
            }
            let shown = (e.hp - old_hp).max(e.sp - old_sp);
            if shown > 0 {
                let pos = self.foe_anim_pos(ei);
                self.push_number(pos, shown.to_string(), NumberKind::Heal);
            }
            return Some(format!("{name} varázsol: {name} +{shown}"));
        }
        let ti = self.retarget_member(target)?;
        let can_act = logic::worst_restriction(&self.members[ti].states, &self.states) != 1;
        let hit = logic::skill_to_hit(
            &skill,
            self.enemies[ei].stats.agility,
            self.members[ti].stats.agility,
            can_act,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
            let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
            self.pending_se.push(BattleSe::Dodge);
            self.push_number(pos, "Miss".to_string(), NumberKind::Miss);
            return Some(format!(
                "{name} varázsol: {} elkerülte",
                self.members[ti].name
            ));
        }
        let base = logic::skill_effect(
            &skill,
            &self.enemies[ei].stats,
            &self.members[ti].stats,
            true,
        );
        let resisted = base > 0
            && skill
                .attributes
                .iter()
                .any(|a| self.members[ti].resist_attributes.contains(a));
        let base = if resisted { (base / 2).max(1) } else { base };
        let effect =
            logic::variance_adjust(base, skill.variance as i32, rng_next(&mut self.rng)).max(0);
        let old_hp = self.members[ti].hp.max(0);
        let dmg = if skill.affect_hp {
            self.hit_member(ti, effect, 0)
        } else {
            0
        };
        if skill.absorb && dmg > 0 {
            let enemy = &mut self.enemies[ei];
            enemy.hp = (enemy.hp + dmg.min(old_hp)).min(enemy.max_hp);
        }
        let sp_lost = if skill.affect_sp && self.members[ti].alive() {
            self.skill_sp_damage(Source::Enemy(ei), Source::Party(ti), effect, skill.absorb)
        } else {
            0
        };
        let mut line = if skill.affect_sp && !skill.affect_hp {
            format!("{name} varázsol: {} -{sp_lost} SP", self.members[ti].name)
        } else {
            format!("{name} varázsol: {} -{dmg}", self.members[ti].name)
        };
        if skill.affect_sp && skill.affect_hp {
            line.push_str(&format!("; -{sp_lost} SP"));
        }
        Some(line)
    }

    /// The screen anchors an enemy `ei`'s cast of `skill` at member `target`
    /// animates over: the casting foe itself for an ally-scope skill (scope 2–4,
    /// which a foe turns on itself), else the targeted member's party slot
    /// (falling back to any living member). Draws no RNG.
    pub(in crate::battle::resolve) fn enemy_skill_anim_anchors(
        &self,
        ei: usize,
        skill: &SkillDef,
        target: usize,
    ) -> Vec<(f32, f32)> {
        if matches!(skill.scope, 2..=4) {
            vec![self.foe_anim_pos(ei)]
        } else {
            let ti = if self.members.get(target).is_some_and(|m| m.alive()) {
                Some(target)
            } else {
                self.living_members().first().copied()
            };
            ti.map(|ti| vec![(self.party_anim_x(ti), PARTY_ANIM_Y)])
                .unwrap_or_default()
        }
    }
}
