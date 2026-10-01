//! Engine-agnostic asset format shared by the converter and game, without
//! dependencies on Bevy or the legacy runtime. Database IDs are 1-based.

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

/// Actor 1's default name from the original database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hero {
    pub name: String,
}

/// A level-triggered skill acquisition from RM2000 `Game_Actor::LearnLevelSkills`.
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

/// Playable actor defaults; HP/SP come from the curves at the starting `level`.
/// Equipment IDs use 0 for an empty slot; dual wielding puts a weapon in `shield`.
/// `face_index` selects a 48×48 cell in the FaceSet's row-major 4×4 grid.
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

/// Skill effects use `physical_rate`/`magical_rate` weights and `variance` on
/// 0–10 scales; `hit` is a percentage. Attribute and state IDs are 1-based.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDef {
    #[serde(default)]
    pub using_message1: String,
    #[serde(default)]
    pub using_message2: String,
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
    /// 0 = normal, 1 = teleport, 2 = escape, 3 = switch.
    #[serde(default)]
    pub skill_type: u32,
    /// 0 = one enemy, 1 = all enemies, 2 = self, 3 = one ally, 4 = all allies.
    #[serde(default)]
    pub scope: u32,
    /// Per-target battle animation; 0 disables it.
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

/// Status condition. `priority` (0–100) controls display and state suppression.
/// After `hold_turn`, `auto_release_prob` is the per-turn recovery percentage;
/// `release_by_damage` is the recovery percentage on a physical hit.
/// Per-turn HP change is `hp_change_val + max_hp * hp_change_max / 100`.
/// Map drain is `hp_change_map_val` HP per `hp_change_map_steps` steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateDef {
    #[serde(default = "default_state_color")]
    pub color: u32,
    #[serde(default)]
    pub message_actor: String,
    #[serde(default)]
    pub message_enemy: String,
    #[serde(default)]
    pub message_already: String,
    #[serde(default)]
    pub message_affected: String,
    #[serde(default)]
    pub message_recovery: String,
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
    /// 0 = unrestricted, 1 = cannot act, 2/3 = random enemy/ally attack.
    pub restriction: u32,
    pub priority: u32,
    pub hold_turn: u32,
    pub auto_release_prob: u32,
    pub release_by_damage: u32,
    /// 0 = lose HP, 1 = gain HP, 2 = no change.
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

fn default_state_color() -> u32 {
    6
}

fn default_hundred() -> u32 {
    100
}

/// Element: `attribute_type` 0 = physical/weapon, 1 = magical. A–E ranks map to
/// damage percentages, from most vulnerable to resistant; C defaults to 100%.
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

/// Enemy defaults and rewards from the database.
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

/// Sprite-sheet effect using a `Battle`/`Battle2` graphic. `scope` is 0 for a
/// target or 1 for the screen; target `position` is 0 = head, 1 = centre, 2 = feet.
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

/// Cells drawn together on one animation tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationFrameDef {
    pub cells: Vec<AnimationCellDef>,
}

/// Placed animation tile: `x`/`y` are screen-pixel offsets and `scale` a percent.
/// Tone channels use 0–200 (100 = neutral); transparency uses 0–100 (0 = opaque).
/// Deleted cells set `valid = false` but retain their slot to preserve later indices.
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

/// Flash/sound cue on a 1-based frame. Empty `se_name` is silent; volume and
/// tempo are percentages. `flash_scope`: 0 = none, 1 = target, 2 = screen.
/// Flash RGB channels use RM2000's 0–31 scale.
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
