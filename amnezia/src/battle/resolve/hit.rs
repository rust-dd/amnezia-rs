//! The shared damage-application primitives: screen anchors, the pending
//! animation/number/blink/SE queues, and the two "apply damage to a battler" steps.

use super::*;

impl Battle {
    /// Queue `anim_id` on `targets` (RM2000 screen offsets from centre, y
    /// downward) for `battle.rs` to play as this tick resolves — one queued
    /// animation for the whole cast, so its sound fires once while its cells and
    /// flashes reach every target. A `0` id (no animation) or an empty target list
    /// is skipped, so an attacker with neither a weapon nor an unarmed animation
    /// plays nothing rather than a stray effect.
    pub(in crate::battle::resolve) fn push_anim(&mut self, anim_id: u32, targets: Vec<(f32, f32)>) {
        if anim_id != 0 && !targets.is_empty() {
            self.pending_anims.push(PendingAnim { anim_id, targets });
        }
    }

    /// The horizontal screen offset for member `ti`'s incoming-hit animation:
    /// centred on 0 and fanned out a little by member index (see
    /// [`PARTY_ANIM_SPREAD`]), since the party isn't drawn in front view.
    pub(in crate::battle::resolve) fn party_anim_x(&self, ti: usize) -> f32 {
        let count = self.members.len().max(1) as f32;
        (ti as f32 - (count - 1.0) / 2.0) * PARTY_ANIM_SPREAD
    }

    /// The screen offset for foe `ti`'s battle animation: its backdrop placement
    /// re-centred on the RM2000 320×240 screen (minus the 160×120 half-extent, y
    /// downward), matching the animation player's coordinates.
    pub(in crate::battle::resolve) fn foe_anim_pos(&self, ti: usize) -> (f32, f32) {
        (
            self.enemies[ti].x as f32 - 160.0,
            self.enemies[ti].y as f32 - 120.0,
        )
    }

    /// Queue a floating number to pop on a battler at `pos` as this action
    /// resolves; `battle::floaters` spawns and rises it. Bevy-free.
    pub(in crate::battle::resolve) fn push_number(
        &mut self,
        pos: (f32, f32),
        text: String,
        kind: NumberKind,
    ) {
        self.pending_numbers.push(PendingNumber { pos, text, kind });
    }

    /// Register a landed blow of `dmg` on foe `ti`: play the enemy-damaged SE, pop
    /// its damage number, owe it a guaranteed whitening blink (RM2000 blinks a
    /// struck sprite every hit, animation-flash or not), and start its death-out if
    /// the blow felled it. The SE fires on any landed blow (even a blocked 0), like
    /// EasyRPG's damage-message substate; a felled foe then adds the kill SE via
    /// [`Battle::start_foe_death`].
    pub(in crate::battle::resolve) fn after_foe_hit(&mut self, ti: usize, dmg: i32) {
        let pos = self.foe_anim_pos(ti);
        self.pending_se.push(BattleSe::EnemyDamaged);
        let (text, kind) = number_for(dmg);
        self.push_number(pos, text, kind);
        self.pending_blinks.push(pos);
        self.start_foe_death(ti, false);
    }

    /// Register a landed blow of `dmg` on member `ti`: play the actor-damaged SE
    /// and pop its damage number at the party slot. Party members have no
    /// front-view sprite, so the number is their whole visual feedback (no blink,
    /// no death-out).
    pub(in crate::battle::resolve) fn after_member_hit(&mut self, ti: usize, dmg: i32) {
        let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
        self.pending_se.push(BattleSe::ActorDamaged);
        let (text, kind) = number_for(dmg);
        self.push_number(pos, text, kind);
    }

    /// Apply `base` damage to enemy `ti` with `var` variance (4 for a physical
    /// blow, the skill's variance for a cast), one draw per hit, then the plain
    /// defending-foe halving (no floor, like a member Defend). Returns the damage
    /// dealt.
    pub(in crate::battle::resolve) fn hit_enemy(&mut self, ti: usize, base: i32, var: i32) -> i32 {
        let roll = rng_next(&mut self.rng);
        let mut dmg = logic::variance_adjust(base, var, roll).max(0);
        if self.enemies[ti].defending {
            dmg = logic::defended(dmg);
        }
        self.enemies[ti].hp -= dmg;
        self.release_states_on_enemy(ti);
        self.after_foe_hit(ti, dmg);
        dmg
    }

    /// Apply `base` damage to member `ti` with `var` variance (4 for a physical
    /// blow, the skill's variance for a cast), one draw per hit, then the member's
    /// own defend halving. Returns the damage dealt.
    pub(in crate::battle::resolve) fn hit_member(&mut self, ti: usize, base: i32, var: i32) -> i32 {
        let roll = rng_next(&mut self.rng);
        let mut dmg = logic::variance_adjust(base, var, roll).max(0);
        if self.members[ti].defending {
            dmg = logic::defended(dmg);
        }
        self.members[ti].hp -= dmg;
        self.release_states_on_member(ti);
        self.after_member_hit(ti, dmg);
        dmg
    }
}
