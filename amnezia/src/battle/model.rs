//! Live combatants, command selection and turn queues. Action resolution lives in
//! [`super::resolve`]; both can be tested without Bevy systems.

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

#[cfg(test)]
pub(in crate::battle) use amnezia_data::{ActorDef, ItemDef, MonsterDef, StateDef, TroopDef};
