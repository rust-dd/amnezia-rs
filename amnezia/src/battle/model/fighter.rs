//! A party member in the fight: live HP/SP, derived stats, and its chosen command.

use super::{Command, Stats};

/// A party member in the fight: live HP/SP, derived stats, and the command it has
/// chosen this round (if any).
pub struct Fighter {
    pub actor_id: u32,
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub sp: i32,
    pub max_sp: i32,
    pub stats: Stats,
    pub defending: bool,
    pub command: Option<Command>,
    /// The equipped weapon's hit and crit rates (percent) and its element id,
    /// captured at build time and consumed by the to-hit / critical / elemental
    /// resolution in [`crate::battle::resolve`]. Empty-handed leaves them `0` / `0`
    /// / `None`; a `0` hit reads as the RM2000 bare-hands 90% default.
    pub weapon_hit: u32,
    pub weapon_crit: u32,
    pub weapon_element: Option<u32>,
    /// The animation this member's normal attack plays on its target: the
    /// equipped weapon's `weapon_animation`, or the actor's `unarmed_animation`
    /// when it has no weapon. `0` means "no animation" and plays nothing.
    pub(in crate::battle) attack_animation: u32,
    /// This fighter's active status effects as `(state_id, turns_held)` pairs; the
    /// turn count drives [`crate::battle::logic::tick_recovery`]'s hold-then-wear-off
    /// schedule.
    pub states: Vec<(u32, u32)>,
    /// The skill ids this member knows at its current level (its actor `learnings`
    /// at or below the level), captured at build time. The battle skill command
    /// offers only these, not the whole database.
    pub(in crate::battle) known_skills: Vec<u32>,
    /// The 1-based attribute (element) ids this member's equipped gear guards
    /// against, unioned across its five slots at build time. A matching enemy
    /// skill's damage is halved once in [`crate::battle::resolve`].
    pub(in crate::battle) resist_attributes: Vec<u32>,
}

impl Fighter {
    pub fn alive(&self) -> bool {
        self.hp > 0
    }
}
