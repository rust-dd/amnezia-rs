use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn plan_attack(
        &mut self,
        source: Source,
        target: Source,
        charged: bool,
    ) -> TargetPlan {
        let (hit, critical, attributes) = match source {
            Source::Party(i) => {
                let actor = &self.members[i];
                (
                    actor.weapon_hit,
                    logic::critical_chance(actor.base_critical_denominator, actor.weapon_crit),
                    actor.weapon_attributes.clone(),
                )
            }
            Source::Enemy(i) => (
                90,
                logic::critical_chance(self.enemies[i].base_critical_denominator, 0),
                Vec::new(),
            ),
        };
        let can_act = self.state_restriction(target) != 1;
        let hit = logic::to_hit_vs(
            hit * logic::state_hit_ratio(self.battler_states(source), &self.states) / 100,
            self.battler_stats(source).agility,
            self.battler_stats(target).agility,
            can_act,
        );
        let gear = self.battler_equipment_effects(target);
        let mut plan = TargetPlan {
            failure: 3,
            ..Default::default()
        };
        if !self.skill_roll(gear.physical_hit(hit, can_act)) {
            return plan;
        }
        let opposed = std::mem::discriminant(&source) != std::mem::discriminant(&target);
        let chance = if opposed {
            gear.critical_chance(critical)
        } else {
            0
        };
        plan.critical = self.skill_roll(chance as i32);
        let base = logic::physical_damage(
            self.battler_stats(source).attack,
            self.battler_stats(target).defense,
        );
        let base = self.battler_attribute_damage(base, target, &attributes);
        let base = if plan.critical {
            logic::critical_damage(base)
        } else if charged {
            base * 2
        } else {
            base
        };
        let damage = self.variance(base, 4).max(0);
        let damage = if self.defending(target) {
            logic::defended(damage)
        } else {
            damage
        };
        plan.success = true;
        plan.hp = Some(-damage);
        if self.battler_hp(target) > damage {
            let mut states = self.planning_states(target);
            self.plan_releases(&mut plan, &mut states, 100);
            self.plan_weapon_states(source, target, &mut plan, &mut states);
        }
        plan
    }

    pub(in crate::battle::resolve) fn plan_explosion(
        &mut self,
        source: Source,
        target: Source,
    ) -> TargetPlan {
        let base = (self.battler_stats(source).attack as i32
            - self.battler_stats(target).defense as i32 / 2)
            .max(0);
        let damage = self.variance(base, 4).max(0);
        let damage = if self.defending(target) {
            logic::defended(damage)
        } else {
            damage
        };
        let mut plan = TargetPlan {
            success: true,
            hp: Some(-damage),
            ..Default::default()
        };
        if self.battler_hp(target) > damage {
            self.plan_releases(&mut plan, &mut self.planning_states(target), 100);
        }
        plan
    }
}
