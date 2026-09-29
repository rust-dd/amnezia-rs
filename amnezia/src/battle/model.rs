//! The live battle state and its turn-flow: the party fighters, the enemy foes,
//! the per-round command bookkeeping, and the agility-ordered turn queue. Building
//! the encounter and marshalling commands live here; the resolution mathematics
//! (applying actions, end checks, rewards, flee) live in the sibling [`resolve`]
//! module. Both are impure-but-Bevy-free and unit-tested directly, so the Bevy
//! systems in `battle.rs`/`input.rs` stay thin drivers.
//!
//! The state is grouped by responsibility into submodules and re-exported here so
//! callers keep using `model::<name>` unchanged: [`fighter`] and [`foe`] (the two
//! sides' live combatants), [`queue`] (the command/turn-queue and pending-effect
//! vocabulary), [`battle`] (the [`Battle`] resource and its turn-flow), and
//! [`build`] ([`Battle::build`]).
//!
//! [`resolve`]: super::resolve

mod battle;
mod build;
mod fighter;
mod foe;
mod queue;

#[cfg(test)]
pub(in crate::battle) mod testkit;

#[cfg(test)]
mod tests;

pub use battle::{Battle, MenuLevel, Phase};
pub use fighter::Fighter;
pub use foe::Foe;
pub use queue::{Action, Command, Source};

pub(in crate::battle) use super::BattleOutcome;
pub(in crate::battle) use battle::rng_next;
pub(in crate::battle) use foe::Dying;
pub(in crate::battle) use queue::{BattleSe, HitKind, HitReport, PendingAnim};

pub(in crate::battle) use super::logic::{self, Stats};
pub(crate) use crate::progression::Progression;
pub(crate) use crate::vitals::Vitals;

// Data-def types re-exported only for the model unit tests' and testkit's glob.
#[cfg(test)]
pub(in crate::battle) use amnezia_data::{ActorDef, ItemDef, MonsterDef, StateDef, TroopDef};
