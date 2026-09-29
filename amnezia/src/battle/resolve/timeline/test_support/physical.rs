use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn plan_strike(&mut self, pi: usize, ti: usize) -> Strike {
        let plan = self.plan_attack(Source::Party(pi), Source::Enemy(ti), false);
        strike(&plan)
    }

    pub(in crate::battle::resolve) fn strike_enemy(&mut self, pi: usize, ti: usize) -> Strike {
        let plan = self.plan_attack(Source::Party(pi), Source::Enemy(ti), false);
        let result = strike(&plan);
        self.push_anim(
            self.members[pi].attack_animation,
            vec![self.foe_anim_pos(ti)],
        );
        self.test_strike(Source::Party(pi), Source::Enemy(ti), plan);
        result
    }

    fn test_strike(&mut self, source: Source, target: Source, plan: TargetPlan) {
        let stage = if plan.critical {
            Stage::Critical
        } else {
            Stage::Apply
        };
        self.run_effect(source, target, plan, stage);
    }

    pub(in crate::battle::resolve) fn resolve_strike_impact(
        &mut self,
        pi: usize,
        ti: usize,
        outcome: Strike,
    ) {
        let plan = match outcome {
            Strike::Miss => TargetPlan {
                failure: 3,
                ..Default::default()
            },
            Strike::Hit { dmg, crit } => {
                let mut plan = self.test_damage_plan(Source::Enemy(ti), dmg, 100, true);
                plan.critical = crit;
                if self.enemies[ti].hp > dmg {
                    let mut states = self.planning_states(Source::Enemy(ti));
                    self.plan_weapon_states(
                        Source::Party(pi),
                        Source::Enemy(ti),
                        &mut plan,
                        &mut states,
                    );
                }
                plan
            }
        };
        self.test_strike(Source::Party(pi), Source::Enemy(ti), plan);
    }

    pub(in crate::battle::resolve) fn land_strike(&mut self, pi: usize, ti: usize, dmg: i32) {
        self.resolve_strike_impact(pi, ti, Strike::Hit { dmg, crit: false });
    }

    pub(in crate::battle::resolve) fn enemy_strike_member(
        &mut self,
        ei: usize,
        ti: usize,
    ) -> Option<i32> {
        let charged = std::mem::take(&mut self.enemies[ei].charging);
        let plan = self.plan_attack(Source::Enemy(ei), Source::Party(ti), charged);
        let damage = plan.hp.map(|hp| -hp);
        self.test_strike(Source::Enemy(ei), Source::Party(ti), plan);
        damage
    }

    pub(in crate::battle::resolve) fn confused_attack(&mut self, source: Source, target: Source) {
        let charged = match source {
            Source::Enemy(i) => std::mem::take(&mut self.enemies[i].charging),
            _ => false,
        };
        let plan = self.plan_attack(source, target, charged);
        if let Source::Party(i) = source {
            self.push_anim_mode(
                self.members[i].attack_animation,
                vec![self.battler_pos(target)],
                matches!(target, Source::Party(_)),
            );
        }
        self.test_strike(source, target, plan);
    }

    pub(in crate::battle::resolve) fn land_ally_strike(
        &mut self,
        source: Source,
        target: Source,
        damage: Option<i32>,
    ) {
        let mut plan = damage.map_or(
            TargetPlan {
                failure: 3,
                ..Default::default()
            },
            |damage| self.test_damage_plan(target, damage, 100, true),
        );
        if damage.is_some_and(|damage| self.battler_hp(target) > damage) {
            let mut states = self.planning_states(target);
            self.plan_weapon_states(source, target, &mut plan, &mut states);
        }
        self.test_strike(source, target, plan);
    }

    fn test_damage_plan(
        &mut self,
        target: Source,
        damage: i32,
        rate: u32,
        normal: bool,
    ) -> TargetPlan {
        let mut plan = TargetPlan {
            success: true,
            hp: Some(-damage),
            absorb_hp: !normal,
            ..Default::default()
        };
        if self.battler_hp(target) > damage {
            let mut states = self.planning_states(target);
            self.plan_releases(&mut plan, &mut states, rate);
        }
        plan
    }

    pub(in crate::battle::resolve) fn hit_battler(
        &mut self,
        target: Source,
        base: i32,
        var: i32,
        rate: u32,
        normal: bool,
    ) -> i32 {
        let damage = self.variance(base, var as u32).max(0);
        let damage = if self.defending(target) {
            logic::defended(damage)
        } else {
            damage
        };
        let plan = self.test_damage_plan(target, damage, rate, normal);
        let source = match target {
            Source::Party(_) => Source::Enemy(0),
            Source::Enemy(_) => Source::Party(0),
        };
        self.run_effect(source, target, plan, Stage::Apply);
        damage
    }

    pub(in crate::battle::resolve) fn hit_member(
        &mut self,
        ti: usize,
        base: i32,
        var: i32,
        rate: u32,
    ) -> i32 {
        self.hit_battler(Source::Party(ti), base, var, rate, true)
    }
}

fn strike(plan: &TargetPlan) -> Strike {
    if plan.success {
        Strike::Hit {
            dmg: -plan.hp.unwrap(),
            crit: plan.critical,
        }
    } else {
        Strike::Miss
    }
}
