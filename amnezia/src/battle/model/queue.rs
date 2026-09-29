//! Turn commands and the feedback queues consumed by the battle scene.

/// A chosen action, from either side, awaiting resolution. Party members choose
/// `Attack`, `Skill`, `Item`, `Defend`, or `Nothing`; the enemy AI reuses `Attack`,
/// `Skill`, `Defend`, and `Nothing`, and adds the RM2000 monster-only basics
/// `DoubleAttack`, `SelfDestruct`, `Escape`, `Charge`, `Observe`, and `DoNothing`.
#[derive(Clone, Copy)]
pub enum Command {
    Attack {
        target: usize,
    },
    Skill {
        skill_id: u32,
        target: usize,
    },
    Item {
        item_id: u32,
        target: usize,
    },
    Defend,
    Nothing,
    Observe,
    /// A deliberate AI no-op still starts an action, unlike a cancelled command.
    DoNothing,
    /// Enemy-only: strike the target twice (two independent hit/damage rolls).
    DoubleAttack {
        target: usize,
    },
    /// Enemy-only: damage every living party member and hide without a reward.
    SelfDestruct,
    /// Enemy-only: flee the fight — the foe leaves without granting a reward.
    Escape,
    /// Enemy-only: gather power so the foe's next physical strike deals double.
    Charge,
}

/// Which side (and index) an action originates from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Party(usize),
    Enemy(usize),
}

/// One entry in the agility-ordered turn queue.
#[derive(Clone, Copy)]
pub struct Action {
    pub source: Source,
    pub kind: Command,
    pub agility: u32,
}

/// One queued battle animation, produced as an action resolves and drained by
/// `battle.rs`'s `drain_pending_anims` into a single `PlayAnimation` overlay
/// message. `anim_id` is the effect id; `targets` are the RM2000 screen offsets
/// from the screen centre (y downward) of every battler the cast hits — one entry
/// for a single-target strike, several for a multi-target skill — so the effect's
/// sound plays once for the whole cast while its cells and flashes land on each
/// target.
#[derive(Clone)]
pub(in crate::battle) struct PendingAnim {
    pub anim_id: u32,
    pub targets: Vec<(f32, f32)>,
    pub sound_only: bool,
}

/// Hit classification for diagnostic traces and resolution regressions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::battle) enum HitKind {
    Damage,
    Heal,
    Miss,
}

/// A resolved hit for diagnostics; it does not create a visual effect.
#[derive(Clone)]
pub(in crate::battle) struct HitReport {
    pub pos: (f32, f32),
    pub text: String,
    pub kind: HitKind,
}

/// A System-defined battle sound effect queued as an action resolves, drained by
/// `battle.rs`'s `drain_pending_se` into an `AudioRequest` whose asset name comes
/// from the loaded `SystemDef`. The resolution stays Bevy- and data-free by
/// naming only the effect's *role* here; the drain maps it to the configured
/// sound. Mirrors the EasyRPG `SePlay(GetSystemSE(...))` sites.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::battle) enum BattleSe {
    UseItem,
    /// A blow landed on a foe (RM2000 `SFX_EnemyDamage`).
    EnemyDamaged,
    /// A blow landed on a party member (RM2000 `SFX_AllyDamage`).
    ActorDamaged,
    /// An attack was evaded (RM2000 `SFX_Evasion`).
    Dodge,
    /// A foe was felled (RM2000 `SFX_EnemyKill`).
    EnemyDefeated,
    /// The party attempted to flee (RM2000 `SFX_Escape`).
    Escape,
    /// A command-menu cursor move (RM2000 `SFX_Cursor`).
    Cursor,
    /// A command-menu selection confirmed (RM2000 `SFX_Decision`).
    Decision,
    /// A command-menu cancel / step back (RM2000 `SFX_Cancel`).
    Cancel,
    /// An empty or invalid command-menu confirm (RM2000 `SFX_Buzzer`).
    Buzzer,
}
