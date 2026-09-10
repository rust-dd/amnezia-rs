//! Clean intermediate data format shared between the asset converter and the
//! game.
//!
//! The converter (writer) and the Bevy game (reader) agree on these
//! `serde`-serialisable types. This crate deliberately carries no RPG Maker
//! 2000 or Bevy dependency, so the on-disk format stays engine-agnostic and
//! the shipped game binary inherits nothing from the legacy runtime.

use serde::{Deserialize, Serialize};

mod item;
mod map;
mod map_info;
mod move_route;
mod panorama;
mod system;
mod terms;
mod terrain;
mod troop;
mod vehicle;

pub use item::ItemDef;
pub use map::{Chipset, CommonEvent, Event, EventCommand, EventCondition, EventPage, Map, Start};
pub use map_info::{MapBgm, MapInfoDef, resolve_map_bgm};
pub use move_route::{MoveCommandDef, MoveRouteDef};
pub use panorama::PanoramaDef;
pub use system::{MusicDef, SoundDef, SystemDef};
pub use terms::{ShopTerms, TermsDef};
pub use terrain::TerrainDef;
pub use troop::{TroopDef, TroopMemberDef, TroopPageConditionDef, TroopPageDef};
pub use vehicle::VehicleDef;

/// The hero's name (actor 1's default name from the original database). The
/// game substitutes it into the `\N[k]` message control code at display time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hero {
    pub name: String,
}

/// One entry in an actor's skill-learning list: the `level` at which the actor
/// learns skill `skill_id`. A member's known skills are every `Learning` whose
/// `level` is at or below its current level (RM2000 `Game_Actor::LearnLevelSkills`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Learning {
    pub level: u32,
    pub skill_id: u32,
}

/// Per-level stat curves for a playable actor (level L is index L-1; length == max_level).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorCurves {
    pub max_hp: Vec<u32>,
    pub max_sp: Vec<u32>,
    pub attack: Vec<u32>,
    pub defense: Vec<u32>,
    pub spirit: Vec<u32>,
    pub agility: Vec<u32>,
}

/// A playable actor's definition, read by the status and equip menus and the
/// level-up system: its 1-based id, name and class title, its starting and
/// maximum level, and the HP/SP it begins with (taken from the level-parameter
/// curve at `level`). `curves` holds the full per-level stat tables and
/// `exp_base`/`exp_inflation`/`exp_correction` parameterise the experience curve.
///
/// `weapon`/`shield`/`armor`/`helmet`/`accessory` are the item ids the actor
/// starts equipped with (0 = empty slot); `two_weapons` marks a dual-wielding
/// actor (the shield slot holds a second weapon); `fix_equipment` an actor whose
/// gear can't be changed; and `unarmed_animation` the attack animation id used
/// with no weapon equipped.
///
/// `face_name` names the actor's FaceSet graphic and `face_index` selects its
/// 48×48 portrait cell in that sheet's 4×4 grid (`col = index % 4`,
/// `row = index / 4`); the menu status window draws it beside the member's stats.
///
/// `learnings` is the actor's skill-learning list — the `(level, skill_id)` pairs
/// it learns as it levels up. A member's known skills (shown in the skill menu and
/// usable in battle) are exactly those learnings at or below its current level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorDef {
    #[serde(default)]
    pub character_name: String,
    #[serde(default)]
    pub character_index: u32,
    #[serde(default)]
    pub rename_skill: bool,
    #[serde(default)]
    pub skill_name: String,
    #[serde(default = "default_true")]
    pub critical_hit: bool,
    #[serde(default = "default_critical_denominator")]
    pub critical_hit_chance: u32,
    #[serde(default)]
    pub attribute_ranks: Vec<u8>,
    #[serde(default)]
    pub state_ranks: Vec<u8>,
    pub id: u32,
    pub name: String,
    pub title: String,
    pub level: u32,
    pub max_level: u32,
    pub hp: u32,
    pub sp: u32,
    #[serde(default)]
    pub curves: ActorCurves,
    #[serde(default)]
    pub learnings: Vec<Learning>,
    #[serde(default)]
    pub exp_base: u32,
    #[serde(default)]
    pub exp_inflation: u32,
    #[serde(default)]
    pub exp_correction: u32,
    #[serde(default)]
    pub weapon: u32,
    #[serde(default)]
    pub shield: u32,
    #[serde(default)]
    pub armor: u32,
    #[serde(default)]
    pub helmet: u32,
    #[serde(default)]
    pub accessory: u32,
    #[serde(default)]
    pub two_weapons: bool,
    #[serde(default)]
    pub fix_equipment: bool,
    #[serde(default = "default_animation_id")]
    pub unarmed_animation: u32,
    #[serde(default)]
    pub face_name: String,
    #[serde(default)]
    pub face_index: u32,
}

/// A skill (spell/ability) definition, read by the skill menu and battle
/// system: its 1-based id, name, description, `sp_cost` (SP spent to cast),
/// `power` (base effect magnitude), and `hit` (base success rate, percent).
///
/// The remaining fields carry the RM2000 battle effect. `scope` picks its
/// targets (`0` one enemy, `1` all enemies, `2` self, `3` one ally, `4` all
/// allies) and `skill_type` its family (`0` normal — the only battle-relevant
/// kind — `1` teleport, `2` escape, `3` switch). `animation_id` is the battle
/// animation the skill overlays on each target it resolves against (`0` shows
/// none). `physical_rate`/`magical_rate`
/// (0–10) weight the caster's attack versus spirit in the damage formula and
/// `variance` (0–10) sets how widely the final damage is randomised around the
/// computed amount (RM2000 editor default 4). `affect_hp`/`affect_sp` mark which
/// pool the effect changes and `absorb` whether the caster drains what it deals.
/// `attributes` holds the 1-based element ids the damage is checked against and
/// `affected_states` the 1-based state ids the skill inflicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDef {
    /// ATK, DEF, SPI, AGI effect flags, in database order.
    #[serde(default)]
    pub affect_stats: [bool; 4],
    #[serde(default)]
    pub ignore_defense: bool,
    pub id: u32,
    pub name: String,
    pub description: String,
    pub sp_cost: u32,
    pub power: u32,
    pub hit: u32,
    /// RM2000 miss-message selector; `3` enables physical accuracy modifiers.
    #[serde(default)]
    pub failure_message: u32,
    #[serde(default)]
    pub skill_type: u32,
    #[serde(default)]
    pub scope: u32,
    /// The battle-animation id this skill plays on each target it resolves
    /// against; `0` shows no animation.
    #[serde(default = "default_animation_id")]
    pub animation_id: u32,
    #[serde(default)]
    pub physical_rate: u32,
    #[serde(default)]
    pub magical_rate: u32,
    #[serde(default)]
    pub variance: u32,
    #[serde(default)]
    pub affect_hp: bool,
    #[serde(default)]
    pub affect_sp: bool,
    #[serde(default)]
    pub absorb: bool,
    #[serde(default)]
    pub attributes: Vec<u32>,
    #[serde(default)]
    pub affected_states: Vec<u32>,
}

/// A state (status condition) definition, read by the battle system: its
/// 1-based id, name, and how it constrains and wears off a battler.
/// `restriction` limits actions while it holds (`0` none, `1` can't act,
/// `2` attack an enemy at random, `3` attack an ally at random) and `priority`
/// (0–100) chooses the displayed state and suppresses lower-priority states.
/// Recovery is governed by `hold_turn` (minimum turns held before it can lift),
/// `auto_release_prob` (percent chance per turn to lift afterwards), and
/// `release_by_damage` (percent chance to lift when hit by a physical attack).
///
/// `hp_change_type` says how an HP-changing state moves HP (`0` lose, `1` gain,
/// `2` nothing): each battle turn the battler loses or gains `hp_change_val`
/// flat points plus `hp_change_max` percent of its max HP, while
/// `hp_change_map_steps`/`hp_change_map_val` drain it on the map
/// (`hp_change_map_val` HP per `hp_change_map_steps` steps). All five default to
/// 0 (a zero-amount no-op); Poison sets them to bleed HP each battle turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateDef {
    #[serde(default)]
    pub affect_type: u32,
    #[serde(default)]
    pub affect_stats: [bool; 4],
    #[serde(default = "default_hundred")]
    pub reduce_hit_ratio: u32,
    #[serde(default)]
    pub restrict_skill: bool,
    #[serde(default)]
    pub restrict_skill_level: u32,
    #[serde(default)]
    pub restrict_magic: bool,
    #[serde(default)]
    pub restrict_magic_level: u32,
    #[serde(default)]
    pub sp_change_type: u32,
    #[serde(default)]
    pub sp_change_max: u32,
    #[serde(default)]
    pub sp_change_val: u32,
    #[serde(default = "default_state_rates")]
    pub rates: [u32; 5],
    #[serde(default)]
    pub persistence: u32,
    pub id: u32,
    pub name: String,
    pub restriction: u32,
    pub priority: u32,
    pub hold_turn: u32,
    pub auto_release_prob: u32,
    pub release_by_damage: u32,
    #[serde(default)]
    pub hp_change_type: u32,
    #[serde(default)]
    pub hp_change_max: u32,
    #[serde(default)]
    pub hp_change_val: u32,
    #[serde(default)]
    pub hp_change_map_steps: u32,
    #[serde(default)]
    pub hp_change_map_val: u32,
}

fn default_state_rates() -> [u32; 5] {
    [100, 80, 60, 30, 0]
}

fn default_hundred() -> u32 {
    100
}

/// An attribute (element) definition, read by the battle system: its 1-based
/// id, name, whether damage carrying it is physical or magical
/// (`attribute_type`: `0` physical/weapon, `1` magical), and the five damage
/// percentages applied by a target's A–E resistance rank (`a_rate` most
/// vulnerable through `e_rate` most resistant; `c_rate` is the neutral 100%).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeDef {
    pub id: u32,
    pub name: String,
    pub attribute_type: u32,
    pub a_rate: u32,
    pub b_rate: u32,
    pub c_rate: u32,
    pub d_rate: u32,
    pub e_rate: u32,
}

/// A monster's definition, read by the battle system: its 1-based id, name, the
/// combat stats (`max_hp`, `max_sp`, `attack`, `defense`, `spirit`, `agility`),
/// and the `exp`/`gold` reward for defeating it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterDef {
    #[serde(default)]
    pub battler_hue: i32,
    #[serde(default)]
    pub drop_id: u32,
    #[serde(default = "default_hundred")]
    pub drop_prob: u32,
    #[serde(default)]
    pub critical_hit: bool,
    #[serde(default = "default_critical_denominator")]
    pub critical_hit_chance: u32,
    pub id: u32,
    pub name: String,
    pub battler: String,
    pub max_hp: u32,
    pub max_sp: u32,
    pub attack: u32,
    pub defense: u32,
    pub spirit: u32,
    pub agility: u32,
    pub exp: u32,
    pub gold: u32,
    #[serde(default)]
    pub attribute_ranks: Vec<u8>,
    #[serde(default)]
    pub state_ranks: Vec<u8>,
    #[serde(default)]
    pub actions: Vec<EnemyActionDef>,
}

/// A weighted enemy action with a condition and post-action switch changes.
/// Turn conditions use `condition_min` as period and `condition_max` as offset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnemyActionDef {
    pub kind: u32,
    pub basic: u32,
    pub skill_id: u32,
    pub enemy_id: u32,
    pub condition_type: u32,
    pub condition_min: u32,
    pub condition_max: u32,
    pub priority: u32,
    pub switch_id: u32,
    pub switch_on: bool,
    pub switch_on_id: u32,
    pub switch_off: bool,
    pub switch_off_id: u32,
}

impl Default for EnemyActionDef {
    fn default() -> Self {
        Self {
            kind: 0,
            basic: 1,
            skill_id: 1,
            enemy_id: 1,
            condition_type: 0,
            condition_min: 0,
            condition_max: 0,
            priority: 50,
            switch_id: 1,
            switch_on: false,
            switch_on_id: 1,
            switch_off: false,
            switch_off_id: 1,
        }
    }
}

/// A battle-animation definition (see `lcf::Animation`), read by the battle
/// system to overlay a sprite-sheet effect when a skill or attack resolves: its
/// 1-based `id`, `name`, the `animation_name` `Battle`/`Battle2` graphic base
/// name, `scope` (`0` one target, `1` the whole screen), `position` (the
/// vertical anchor on the target: `0` head, `1` centre, `2` feet), the per-tick
/// `frames`, and the flash / sound `timings`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationDef {
    pub id: u32,
    pub name: String,
    pub animation_name: String,
    pub scope: u32,
    pub position: u32,
    pub frames: Vec<AnimationFrameDef>,
    pub timings: Vec<AnimationTimingDef>,
}

/// One frame of an animation (see `lcf::AnimationFrame`): the sprite-sheet
/// `cells` drawn together for that tick of the effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationFrameDef {
    pub cells: Vec<AnimationCellDef>,
}

/// One placed sprite-sheet tile within an animation frame (see
/// `lcf::AnimationCell`): `valid` is liblcf's per-cell flag (default `true`) — an
/// editor-deleted cell clears it, keeping its slot so later cells hold their
/// index, and the renderer skips a `valid == false` cell. `cell_id` selects the
/// tile from the animation's graphic, `x`/`y` offset it from the anchor in screen
/// pixels, `scale` is a zoom percent (`100` = full size), the four `tone_*`
/// channels tint it on RM2000's `0..=200` scale (`100` = neutral), and
/// `transparency` is a `0..=100` percent (`0` = opaque).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationCellDef {
    #[serde(default = "default_true")]
    pub valid: bool,
    pub cell_id: u32,
    pub x: i32,
    pub y: i32,
    pub scale: u32,
    pub tone_red: i32,
    pub tone_green: i32,
    pub tone_blue: i32,
    pub tone_gray: i32,
    pub transparency: u32,
}

fn default_true() -> bool {
    true
}

fn default_animation_id() -> u32 {
    1
}

fn default_critical_denominator() -> u32 {
    30
}

fn default_audio_level() -> u32 {
    100
}

/// A frame-timed flash and sound effect on an animation's timeline (see
/// `lcf::AnimationTiming`): `frame` is the 1-based frame it fires on, `se_name`
/// the sound-effect file under `audio/Sound/` (empty = silent) played at
/// `se_volume` (`0..=100`) and percent `se_tempo` (`100` = normal),
/// `flash_scope` selects what flashes (`0` nothing, `1` the target, `2` the whole
/// screen), `flash_red`/`flash_green`/`flash_blue` the flash colour on RM2000's
/// `0..=31` scale, and `flash_power` its strength. `se_volume`/`se_tempo` carry a
/// `serde` default of `100` so timelines written before they existed still load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationTimingDef {
    pub frame: u32,
    pub se_name: String,
    #[serde(default = "default_audio_level")]
    pub se_volume: u32,
    #[serde(default = "default_audio_level")]
    pub se_tempo: u32,
    pub flash_scope: u32,
    pub flash_red: u32,
    pub flash_green: u32,
    pub flash_blue: u32,
    pub flash_power: u32,
}
