//! A party member in the fight: live HP/SP, derived stats, and its chosen command.

use super::{Command, Stats};

/// A party member in the fight: live HP/SP, derived stats, and the command it has
/// chosen this round (if any).
pub struct Fighter {
    pub actor_id: u32,
    pub level: u32,
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub sp: i32,
    pub max_sp: i32,
    pub stats: Stats,
    pub(in crate::battle) stat_modifiers: [i32; 4],
    pub defending: bool,
    pub command: Option<Command>,
    /// Equipped weapon hit/critical percentages and offensive attributes.
    /// Empty-handed defaults to 90% hit; an explicit weapon zero stays zero.
    pub weapon_hit: u32,
    pub weapon_crit: u32,
    pub base_critical_denominator: Option<u32>,
    pub weapon_attributes: Vec<u32>,
    /// The animation this member's normal attack plays on its target: the
    /// equipped weapon's `weapon_animation`, or the actor's `unarmed_animation`
    /// when it has no weapon. `0` means "no animation" and plays nothing.
    pub(in crate::battle) attack_animation: u32,
    /// This fighter's active status effects as `(state_id, turns_held)` pairs; the
    /// turn count drives [`crate::battle::logic::tick_recovery`]'s hold-then-wear-off
    /// schedule.
    pub states: Vec<(u32, u32)>,
    pub(in crate::battle) state_ranks: Vec<u8>,
    pub(in crate::battle) attribute_ranks: Vec<u8>,
    pub(in crate::battle) state_guards: Vec<(u32, u32)>,
    /// The skill ids this member knows at its current level (its actor `learnings`
    /// at or below the level), captured at build time. The battle skill command
    /// offers only these, not the whole database.
    pub(in crate::battle) known_skills: Vec<u32>,
    /// Armor attributes improve the wearer's rank once, without stacking.
    pub(in crate::battle) resist_attributes: Vec<u32>,
}

impl Fighter {
    pub fn alive(&self) -> bool {
        self.hp > 0
    }
}
