//! Physical strikes: a party member's planned weapon swing and its deferred
//! impact, plus an enemy's normal attack on a member.

use super::*;

impl Battle {
    /// Roll a party member's weapon strike on enemy `ti` to its final damage and
    /// critical flag, in RM2000 order — an agility-adjusted to-hit roll (bare hands
    /// default 90%), then on a hit the weapon's element against the foe's
    /// resistance ranks, a critical that triples, the `var=4` variance, and the
    /// defending-foe halving — *without applying it or showing anything yet*. The
    /// attack animation is queued on the struck foe up front (the swing shows
    /// whether the blow lands); both the "Miss" pop and the landing are deferred
    /// to [`Battle::resolve_strike_impact`], which runs only once the animation
    /// has played. The RNG draw order is identical to a single-shot strike.
    pub(in crate::battle::resolve) fn plan_strike(&mut self, pi: usize, ti: usize) -> Strike {
        let anim = self.members[pi].attack_animation;
        self.push_anim(anim, vec![self.foe_anim_pos(ti)]);
        let base = logic::physical_damage(
            self.battler_stats(Source::Party(pi)).attack,
            self.battler_stats(Source::Enemy(ti)).defense,
        );
        // A foe that cannot act (asleep/paralyzed) is struck with certainty
        // (EasyRPG `CalcNormalAttackToHit` returns 100 vs a do-nothing target).
        let can_act = logic::worst_restriction(&self.enemies[ti].states, &self.states) != 1;
        let hit = logic::to_hit_vs(
            self.members[pi].weapon_hit,
            self.battler_stats(Source::Party(pi)).agility,
            self.battler_stats(Source::Enemy(ti)).agility,
            can_act,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
            return Strike::Miss;
        }
        let element = self.members[pi].weapon_element.unwrap_or(0);
        let base = logic::elemental_damage(
            base,
            element,
            &self.enemies[ti].attribute_ranks,
            &self.attributes,
        );
        let crit = ((rng_next(&mut self.rng) % 100) as u32) < self.members[pi].weapon_crit;
        let base = if crit {
            logic::critical_damage(base)
        } else {
            base
        };
        let roll = rng_next(&mut self.rng);
        let mut dmg = logic::variance_adjust(base, 4, roll).max(0);
        // A defending foe halves the final result — plain `dmg/2`, no floor, so a
        // foe and a member Defend behave alike (EasyRPG `AdjustDamageForDefend`),
        // applied after element/crit/variance.
        if self.enemies[ti].defending {
            dmg = logic::defended(dmg);
        }
        Strike::Hit { dmg, crit }
    }

    /// Land a planned strike of `dmg` on enemy `ti`: subtract the HP, roll its
    /// states' damage wear-off, and pop the damage number, whitening blink, and
    /// death-out (see [`Battle::after_foe_hit`]).
    pub(in crate::battle::resolve) fn land_strike(&mut self, ti: usize, dmg: i32) {
        self.enemies[ti].hp = (self.enemies[ti].hp - dmg).max(0);
        self.release_states_on_enemy(ti);
        self.after_foe_hit(ti, dmg);
    }

    /// Apply a member's planned strike `outcome` on foe `ti` once its swing
    /// animation has played (RM2000 `ProcessBattleActionApply`/`Damage`): a miss
    /// pops the dodge SE and "Miss" number, a critical announces on its own beat
    /// and lands its precomputed `dmg` on the next tick ([`Step::CritDamage`]),
    /// and a plain hit lands at once — each logging its line. Draws no RNG, so the
    /// order is unchanged whether this runs inline or deferred behind the hold.
    pub(in crate::battle::resolve) fn resolve_strike_impact(
        &mut self,
        _pi: usize,
        ti: usize,
        outcome: Strike,
    ) {
        let enemy = self.enemies[ti].name.clone();
        match outcome {
            // RM2000 concatenates the target name with the term: a dodge reads
            // "<foe> <dodge>", a hit "<foe> <value><enemy_damaged>", and a critical
            // announces the standalone term on its own beat.
            Strike::Miss => {
                let pos = self.foe_anim_pos(ti);
                self.pending_se.push(BattleSe::Dodge);
                self.push_number(pos, "Miss".to_string(), NumberKind::Miss);
                self.log
                    .push(format!("{enemy}{}", crate::i18n::tr(&self.text.dodge)));
            }
            Strike::Hit { dmg, crit: false } => {
                self.land_strike(ti, dmg);
                self.log.push(format!(
                    "{enemy} {dmg}{}",
                    crate::i18n::tr(&self.text.enemy_damaged)
                ));
            }
            Strike::Hit { dmg, crit: true } => {
                // RM2000 `ProcessBattleActionCritical`: announce the critical on
                // its own beat, then land the (already rolled) blow next tick.
                self.steps.push_back(Step::CritDamage { ti, dmg });
                self.log.push(crate::i18n::tr(&self.text.enemy_critical));
            }
        }
    }

    /// Resolve a member's weapon strike on enemy `ti` in one shot: plan it, then
    /// apply its impact at once. The [`Battle::apply`] attack path defers the
    /// impact behind the swing animation instead, so this atomic form only serves
    /// the strike unit tests.
    #[cfg(test)]
    pub(in crate::battle::resolve) fn strike_enemy(&mut self, pi: usize, ti: usize) -> Strike {
        let outcome = self.plan_strike(pi, ti);
        self.resolve_strike_impact(pi, ti, outcome);
        outcome
    }

    /// One enemy `ei` physical strike on member `ti`: an agility-adjusted to-hit
    /// roll off the RM2000 90% bare-hands base that returns `None` on a miss, else
    /// the dealt damage via `hit_member` (its variance and the member's own defend
    /// halving). A pending charge-up doubles the blow; it is spent on the swing
    /// whether or not the blow lands, so the foe's next strike is normal again. An
    /// rpg2k enemy normal attack plays no animation, so none is queued here (a
    /// weapon/unarmed animation on a party strike is a member-side concern).
    pub(in crate::battle::resolve) fn enemy_strike_member(
        &mut self,
        ei: usize,
        ti: usize,
    ) -> Option<i32> {
        let charged = self.enemies[ei].charging;
        self.enemies[ei].charging = false;
        // A member that cannot act is struck with certainty (EasyRPG
        // `CalcNormalAttackToHit` returns 100 vs a do-nothing target).
        let can_act = logic::worst_restriction(&self.members[ti].states, &self.states) != 1;
        let hit = logic::to_hit_vs(
            logic::effective_hit(None),
            self.battler_stats(Source::Enemy(ei)).agility,
            self.battler_stats(Source::Party(ti)).agility,
            can_act,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
            let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
            self.pending_se.push(BattleSe::Dodge);
            self.push_number(pos, "Miss".to_string(), NumberKind::Miss);
            return None;
        }
        let mut base = logic::physical_damage(
            self.battler_stats(Source::Enemy(ei)).attack,
            self.battler_stats(Source::Party(ti)).defense,
        );
        if charged {
            base *= 2;
        }
        Some(self.hit_member(ti, base, 4))
    }
}
