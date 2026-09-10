//! A live enemy in the fight and the death/explosion fade playing out on it.

use super::Stats;
use amnezia_data::EnemyActionDef;

/// A death (`SetDeathTimer`) or self-destruct explosion (`SetExplodeTimer`)
/// playing out on a foe's sprite before it leaves the field: `elapsed` advances in
/// real time toward `secs`, and `explode` drives the RM2000 zoom-and-fade rather
/// than the plain fade. While any foe's [`Dying`] runs, `resolve_tick` holds so the
/// beat is seen; `battle::scene` reads it to drive the sprite's alpha and zoom.
pub(in crate::battle) struct Dying {
    pub elapsed: f32,
    pub secs: f32,
    pub explode: bool,
}

/// A live enemy in the fight: current HP/SP, stats, reward, and its position on the
/// battle backdrop (RM2000 320×240 pixel space).
pub struct Foe {
    pub name: String,
    pub battler: String,
    pub battler_hue: i32,
    pub hp: i32,
    /// This foe's starting (maximum) HP, kept so HP-percent AI conditions resolve.
    pub max_hp: i32,
    pub sp: i32,
    pub max_sp: i32,
    pub(in crate::battle) base_critical_denominator: Option<u32>,
    pub stats: Stats,
    pub(in crate::battle) stat_modifiers: [i32; 4],
    pub exp: u32,
    pub gold: u32,
    pub(in crate::battle) drop_id: u32,
    pub(in crate::battle) drop_prob: u32,
    pub x: u32,
    pub y: u32,
    /// This foe's per-attribute damage ranks (0=A … 4=E), copied from its
    /// `MonsterDef`. The vector is truncated, so ids past its end read neutral C.
    pub attribute_ranks: Vec<u8>,
    /// This foe's per-state affliction ranks, copied from its `MonsterDef`, for
    /// the status-infliction chance.
    pub state_ranks: Vec<u8>,
    /// This foe's active status effects as `(state_id, turns_held)` pairs; the
    /// turn count drives [`crate::battle::logic::tick_recovery`]'s hold-then-wear-off
    /// schedule.
    pub states: Vec<(u32, u32)>,
    /// Whether this foe took the RM2000 Defend stance on its last turn; it halves
    /// incoming damage in [`crate::battle::resolve`] until
    /// [`crate::battle::model::Battle::new_round`] clears it.
    pub(in crate::battle) defending: bool,
    /// Whether this foe fled the battle (RM2000 monster Escape). It then counts as
    /// gone (see [`Foe::alive`]) but, unlike a defeated foe, grants no reward.
    pub(in crate::battle) fled: bool,
    /// Whether this foe gathered power (RM2000 Charge): its next physical strike
    /// deals double, and the flag is consumed on that strike.
    pub(in crate::battle) charging: bool,
    /// This foe's RM2000 battle-AI action list, consulted each round to choose
    /// its command (cast a skill, defend, or attack on turn/HP conditions).
    pub actions: Vec<EnemyActionDef>,
    pub(in crate::battle) switch_on_after_action: Option<u32>,
    pub(in crate::battle) switch_off_after_action: Option<u32>,
    /// A death or self-destruct fade playing out on this foe's sprite before it
    /// is cleared from view (see [`Dying`]); `None` until the foe is slain.
    pub(in crate::battle) dying: Option<Dying>,
}

impl Foe {
    /// Whether this foe is still in the fight: living HP and not fled. A fled foe
    /// (RM2000 Escape) counts as gone, so it drops out of targeting and the
    /// living-enemy list and grants no reward.
    pub fn alive(&self) -> bool {
        self.hp > 0 && !self.fled
    }

    /// This foe's damage rank (0=A … 4=E) against attribute `attr_id`. Ids past
    /// the truncated `attribute_ranks` vector — and the non-elemental id `0` —
    /// read neutral C (`2`).
    #[allow(dead_code)]
    pub fn attribute_rank(&self, attr_id: u32) -> u8 {
        attr_id
            .checked_sub(1)
            .and_then(|i| self.attribute_ranks.get(i as usize).copied())
            .unwrap_or(2)
    }
}
