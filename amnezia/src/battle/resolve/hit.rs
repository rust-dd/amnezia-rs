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
}
