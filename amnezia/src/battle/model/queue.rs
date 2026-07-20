//! The turn-queue vocabulary — chosen commands, their source and agility-ordered
//! entries, the deferred resolution sub-steps — and the per-tick pending effects
//! (animations, floating numbers, and sound effects) the resolution enqueues.

/// A chosen action, from either side, awaiting resolution. Party members choose
/// `Attack`, `Skill`, `Item`, `Defend`, or `Nothing`; the enemy AI reuses `Attack`,
/// `Skill`, `Defend`, and `Nothing`, and adds the RM2000 monster-only basics
/// `DoubleAttack`, `SelfDestruct`, `Escape`, and `Charge`.
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
    /// Enemy-only: strike the target twice (two independent hit/damage rolls).
    DoubleAttack {
        target: usize,
    },
    /// Enemy-only: damage every living party member, then the foe dies.
    SelfDestruct,
    /// Enemy-only: flee the fight — the foe leaves without granting a reward.
    Escape,
    /// Enemy-only: gather power so the foe's next physical strike deals double.
    Charge,
}

/// Which side (and index) an action originates from.
#[derive(Clone, Copy)]
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

/// A deferred sub-step of the action currently resolving, drained one per
/// resolve tick so a multi-target cast staggers its per-target beats and a
/// critical shows its announcement on its own beat before the damage lands.
/// Mirrors RM2000's `ProcessBattleAction` walking its substates each behind its
/// own `SetWait`, without the full substate machine.
#[derive(Clone, Copy)]
pub(in crate::battle) enum Step {
    /// Land caster `pi`'s multi-target skill on one more enemy `ti`.
    HitEnemy { pi: usize, ti: usize, skill_id: u32 },
    /// Apply caster `pi`'s multi-target heal to one more ally `ti`.
    HealAlly { pi: usize, ti: usize, skill_id: u32 },
    /// The damage beat after a "Kritikus!" announcement: land the precomputed
    /// `dmg` of member `pi`'s critical strike on enemy `ti`.
    CritDamage { pi: usize, ti: usize, dmg: i32 },
    /// Apply member `pi`'s planned normal-strike outcome on foe `ti` once its
    /// attack animation has played out: pop the dodge when `miss`, else land
    /// `dmg` — taking the critical announcement beat first when `crit`. RM2000
    /// sequences the swing animation, its wait, then the damage; this is the
    /// deferred damage half (see `resolve::resolve_strike_impact`).
    StrikeImpact {
        pi: usize,
        ti: usize,
        dmg: i32,
        crit: bool,
        miss: bool,
    },
    /// Apply member `pi`'s skill `skill_id` at `target` once its cast animation
    /// has played. The animation was queued up front; this re-runs the cast with
    /// [`crate::battle::model::Battle::suppress_anim`] set so its effect (and RNG
    /// draws) resolve now without queuing the animation a second time.
    CastSkill {
        pi: usize,
        skill_id: u32,
        target: usize,
    },
    /// Apply enemy `ei`'s skill `skill_id` at member `target` once its cast
    /// animation has played — the enemy-side counterpart of [`Step::CastSkill`].
    EnemyCast {
        ei: usize,
        skill_id: u32,
        target: usize,
    },
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
}

/// What a floating battle number represents, which tints it: white HP `Damage`,
/// green `Heal` (an HP or SP restore), and a pale `Miss` for a dodge or a blocked
/// (0-damage) blow. Mirrors RM2000's damage-pop colouring.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::battle) enum NumberKind {
    Damage,
    Heal,
    Miss,
}

/// A floating number queued as an action resolves — the RM2000 damage/heal pop —
/// drained by `battle::floaters` into rising, fading overlay text. `pos` is the
/// target's RM2000 screen offset from centre (y downward): a foe's anim pos or a
/// member's party slot. `text` is the digits (or "Miss") and `kind` its colour.
#[derive(Clone)]
pub(in crate::battle) struct PendingNumber {
    pub pos: (f32, f32),
    pub text: String,
    pub kind: NumberKind,
}

/// A System-defined battle sound effect queued as an action resolves, drained by
/// `battle.rs`'s `drain_pending_se` into an `AudioRequest` whose asset name comes
/// from the loaded `SystemDef`. The resolution stays Bevy- and data-free by
/// naming only the effect's *role* here; the drain maps it to the configured
/// sound. Mirrors the EasyRPG `SePlay(GetSystemSE(...))` sites.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::battle) enum BattleSe {
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
}
