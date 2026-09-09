use super::*;

impl Battle {
    pub(in crate::battle::resolve) fn confused_attack(&mut self, source: Source, target: Source) {
        let (base_hit, attributes, animation, charged) = match source {
            Source::Party(i) => {
                let member = &self.members[i];
                (
                    member.weapon_hit,
                    member.weapon_attributes.clone(),
                    member.attack_animation,
                    false,
                )
            }
            Source::Enemy(i) => {
                let enemy = &mut self.enemies[i];
                let charged = enemy.charging;
                enemy.charging = false;
                (90, Vec::new(), 0, charged)
            }
        };
        let hit = logic::to_hit_vs(
            base_hit * logic::state_hit_ratio(self.battler_states(source), &self.states) / 100,
            self.battler_stats(source).agility,
            self.battler_stats(target).agility,
            self.state_restriction(target) != 1,
        );
        let damage = if self.skill_roll(hit) {
            let base = logic::physical_damage(
                self.battler_stats(source).attack,
                self.battler_stats(target).defense,
            );
            let base = self.battler_attribute_damage(base, target, &attributes);
            let base = if charged { base * 2 } else { base };
            let damage = logic::variance_adjust(base, 4, rng_next(&mut self.rng));
            let defending = match target {
                Source::Party(i) => self.members[i].defending,
                Source::Enemy(i) => self.enemies[i].defending,
            };
            Some(if defending {
                logic::defended(damage)
            } else {
                damage
            })
        } else {
            None
        };
        if animation != 0 {
            self.push_anim(animation, vec![self.battler_pos(target)]);
            self.steps.push_back(Step::AllyStrikeImpact {
                source,
                target,
                damage,
            });
            self.begin_anim_hold();
        } else {
            self.land_ally_strike(source, target, damage);
        }
    }

    pub(in crate::battle::resolve) fn land_ally_strike(
        &mut self,
        source: Source,
        target: Source,
        damage: Option<i32>,
    ) {
        let source_name = self.battler_name(source).to_string();
        let target_name = self.battler_name(target).to_string();
        if let Some(damage) = damage {
            match target {
                Source::Party(i) => self.members[i].hp = (self.members[i].hp - damage).max(0),
                Source::Enemy(i) => self.enemies[i].hp = (self.enemies[i].hp - damage).max(0),
            }
            self.release_states_from_damage(target, 100);
            match target {
                Source::Party(i) => self.after_member_hit(i, damage),
                Source::Enemy(i) => self.after_foe_hit(i, damage),
            }
            self.log.push(format!(
                "{source_name} zavartan lesújt: {target_name} -{damage}"
            ));
        } else {
            self.pending_se.push(BattleSe::Dodge);
            self.push_number(self.battler_pos(target), "Miss".into(), NumberKind::Miss);
            self.log.push(format!(
                "{source_name} zavartan lesújt: {target_name} elkerülte"
            ));
        }
    }
}
