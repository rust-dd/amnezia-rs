//! Turn commands and the feedback queues consumed by the battle scene.

/// Pending party command or enemy AI action.
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

/// One cast across all target centres (y-down screen offsets). Sharing the cast
/// plays its sound once while applying cells/flashes to every target.
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

/// Sound role resolved through `SystemDef` when drained, keeping resolution independent
/// of Bevy audio assets (EasyRPG `SePlay(GetSystemSE(...))`).
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
