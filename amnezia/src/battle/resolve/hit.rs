//! The shared damage-application primitives: screen anchors, the pending
//! animation/report/blink/SE queues, and damage application on both sides.

use super::*;

impl Battle {
    /// Queue `anim_id` on `targets` (RM2000 screen offsets from centre, y
    /// downward) for `battle.rs` to play as this tick resolves — one queued
    /// animation for the whole cast, so its sound fires once while its cells and
    /// flashes reach every target. A `0` id (no animation) or an empty target list
    /// is skipped, so an attacker with neither a weapon nor an unarmed animation
    /// plays nothing rather than a stray effect.
    pub(in crate::battle) fn push_anim(&mut self, anim_id: u32, targets: Vec<(f32, f32)>) {
        self.push_anim_mode(anim_id, targets, false);
    }

    pub(in crate::battle::resolve) fn push_anim_mode(
        &mut self,
        anim_id: u32,
        targets: Vec<(f32, f32)>,
        sound_only: bool,
    ) {
        if anim_id != 0 && !targets.is_empty() {
            self.pending_anims.push(PendingAnim {
                anim_id,
                targets,
                sound_only,
            });
        }
    }

    pub(in crate::battle::resolve) fn push_skill_anim(
        &mut self,
        source: Source,
        skill: &SkillDef,
        target: usize,
    ) {
        let (anchors, sound_only) = match source {
            Source::Party(i) => (
                self.skill_anim_anchors(i, skill, target),
                matches!(skill.scope, 2..=4),
            ),
            Source::Enemy(i) => (
                self.enemy_skill_anim_anchors(i, skill, target),
                !matches!(skill.scope, 2..=4),
            ),
        };
        self.push_anim_mode(skill.animation_id, anchors, sound_only);
    }

    /// Stable diagnostic anchor for an undrawn party slot.
    pub(in crate::battle::resolve) fn party_anim_x(&self, ti: usize) -> f32 {
        let count = self.members.len().max(1) as f32;
        (ti as f32 - (count - 1.0) / 2.0) * PARTY_ANIM_SPREAD
    }

    /// The screen offset for foe `ti`'s battle animation: its backdrop placement
    /// re-centred on the RM2000 320×240 screen (minus the 160×120 half-extent, y
    /// downward), matching the animation player's coordinates.
    pub(in crate::battle) fn foe_anim_pos(&self, ti: usize) -> (f32, f32) {
        (
            self.enemies[ti].x as f32 - 160.0,
            self.enemies[ti].y as f32 - 120.0,
        )
    }

    /// Record the resolved hit for tracing and tests without drawing overlay text.
    pub(in crate::battle::resolve) fn report_hit(
        &mut self,
        pos: (f32, f32),
        text: String,
        kind: HitKind,
    ) {
        self.hit_reports.push(HitReport { pos, text, kind });
    }

    /// Non-absorbing hits sound and blink even at zero damage.
    pub(in crate::battle::resolve) fn after_foe_hit(&mut self, ti: usize, dmg: i32) {
        self.after_battler_hit(Source::Enemy(ti), dmg, true);
    }

    /// Apply KO and damage sound feedback; front view has no party sprites.
    pub(in crate::battle::resolve) fn after_member_hit(&mut self, ti: usize, dmg: i32) {
        self.after_battler_hit(Source::Party(ti), dmg, true);
    }

    fn after_battler_hit(&mut self, target: Source, dmg: i32, normal_impact: bool) {
        if matches!(target, Source::Party(_)) && self.battler_hp(target) <= 0 {
            self.mark_knocked_out(target);
        }
        let pos = self.battler_pos(target);
        if normal_impact {
            self.pending_se.push(match target {
                Source::Party(_) => BattleSe::ActorDamaged,
                Source::Enemy(_) => BattleSe::EnemyDamaged,
            });
        }
        let (text, kind) = damage_report(dmg);
        self.report_hit(pos, text, kind);
        if let Source::Enemy(ti) = target {
            if normal_impact {
                self.pending_blinks.push(pos);
            }
            self.start_foe_death(ti, false);
        }
    }

    /// Absorption shares damage rules but omits normal hit sound and blinking.
    pub(in crate::battle::resolve) fn hit_battler(
        &mut self,
        target: Source,
        base: i32,
        var: i32,
        physical_rate: u32,
        normal_impact: bool,
    ) -> i32 {
        let roll = rng_next(&mut self.rng);
        let mut dmg = logic::variance_adjust(base, var, roll).max(0);
        let (hp, defending) = match target {
            Source::Party(i) => {
                let member = &mut self.members[i];
                (&mut member.hp, member.defending)
            }
            Source::Enemy(i) => {
                let enemy = &mut self.enemies[i];
                (&mut enemy.hp, enemy.defending)
            }
        };
        if defending {
            dmg = logic::defended(dmg);
        }
        *hp = (*hp - dmg).max(0);
        self.release_states_from_damage(target, physical_rate);
        self.after_battler_hit(target, dmg, normal_impact);
        dmg
    }

    /// Apply `base` damage to member `ti` with `var` variance (4 for a physical
    /// blow, the skill's variance for a cast), one draw per hit, then the member's
    /// own defend halving. Returns the damage dealt.
    pub(in crate::battle::resolve) fn hit_member(
        &mut self,
        ti: usize,
        base: i32,
        var: i32,
        physical_rate: u32,
    ) -> i32 {
        self.hit_battler(Source::Party(ti), base, var, physical_rate, true)
    }
}
