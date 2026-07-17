//! Clean intermediate data format shared between the asset converter and the
//! game.
//!
//! The converter (writer) and the Bevy game (reader) agree on these
//! `serde`-serialisable types. This crate deliberately carries no RPG Maker
//! 2000 or Bevy dependency, so the on-disk format stays engine-agnostic and
//! the shipped game binary inherits nothing from the legacy runtime.

use serde::{Deserialize, Serialize};

/// A converted map: the chipset it uses, its dimensions in tiles, and the two
/// tile layers (each `width * height` tile ids, row-major). `lower` is the
/// ground layer, `upper` the overlay layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Map {
    pub chipset_id: u32,
    pub width: u32,
    pub height: u32,
    pub lower: Vec<u16>,
    pub upper: Vec<u16>,
    pub events: Vec<Event>,
}

/// A map event: its id, tile position, name, and pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub name: String,
    pub pages: Vec<EventPage>,
}

/// One page of an event: its trigger, graphic, and command list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventPage {
    pub trigger: u32,
    pub graphic_name: String,
    pub graphic_index: u32,
    pub layer: u32,
    pub condition: EventCondition,
    pub commands: Vec<EventCommand>,
}

/// A page's activation condition (see `lcf::EventCondition`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventCondition {
    pub flags: u32,
    pub switch_a: u32,
    pub switch_b: u32,
    pub variable_id: u32,
    pub variable_value: u32,
    pub item_id: u32,
    pub actor_id: u32,
}

/// One event command: RM2000 opcode, nesting indent, string, and int params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventCommand {
    pub code: u32,
    pub indent: u32,
    pub string: String,
    pub params: Vec<i32>,
}

/// A common event (global event script), read by the interpreter: its 1-based
/// id, name, `trigger` (0 = call, 1 = autostart, 2 = parallel), the `switch_id`
/// gating an autostart/parallel event, and its command list. Unlike a map
/// event, it belongs to no map and its commands run in the global scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonEvent {
    pub id: u32,
    pub name: String,
    pub trigger: u32,
    pub switch_id: u32,
    pub commands: Vec<EventCommand>,
}

/// A chipset entry: its 1-based id (matching a map's `chipset_id`) and the
/// base name of its graphic under `graphics/ChipSet/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chipset {
    pub id: u32,
    pub graphic: String,
    pub passages_down: Vec<u8>,
    pub passages_up: Vec<u8>,
}

/// The game's starting party position: which map, and the tile within it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Start {
    pub map_id: u32,
    pub x: u32,
    pub y: u32,
}

/// The hero's name (actor 1's default name from the original database). The
/// game substitutes it into the `\N[k]` message control code at display time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hero {
    pub name: String,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorDef {
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
    #[serde(default)]
    pub unarmed_animation: u32,
}

/// An item's definition, read by the shop, item, and equip menus and the
/// use-item systems: its 1-based id, name, description, category (`item_type`:
/// 0 normal, 1 weapon, 2 shield, 3 armor, 4 helmet, 5 accessory, 6 medicine,
/// 7 book, 8 material, 9 special, 10 switch), and buy price.
///
/// The use-effect fields apply when the item is consumed (a medicine, or a
/// normal item used from the menu): `recover_hp`/`recover_sp` restore a fixed
/// amount, `recover_hp_rate`/`recover_sp_rate` a percentage of the maximum,
/// `cure_states` lists the 1-based state ids it lifts, `scope` targets one ally
/// (`0`) or the whole party (`1`), `only_field` marks it usable only from the
/// map menu, and `uses` is the number of uses before it is consumed (`0` =
/// unlimited).
///
/// The equipment fields apply to gear (types 1–5): `atk`/`def`/`spi`/`agi` are
/// the stat bonuses, `attribute_defense`/`state_defense` the 1-based attribute
/// and state ids the gear resists or guards against, `two_handed` marks a
/// two-handed weapon, `hit`/`crit` its hit and critical rates (percent), and
/// `weapon_animation` its attack animation id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub item_type: u32,
    pub price: u32,
    #[serde(default)]
    pub recover_hp: u32,
    #[serde(default)]
    pub recover_hp_rate: u32,
    #[serde(default)]
    pub recover_sp: u32,
    #[serde(default)]
    pub recover_sp_rate: u32,
    #[serde(default)]
    pub cure_states: Vec<u32>,
    #[serde(default)]
    pub scope: u32,
    #[serde(default)]
    pub only_field: bool,
    #[serde(default)]
    pub uses: u32,
    #[serde(default)]
    pub atk: u32,
    #[serde(default)]
    pub def: u32,
    #[serde(default)]
    pub spi: u32,
    #[serde(default)]
    pub agi: u32,
    #[serde(default)]
    pub attribute_defense: Vec<u32>,
    #[serde(default)]
    pub state_defense: Vec<u32>,
    #[serde(default)]
    pub two_handed: bool,
    #[serde(default)]
    pub hit: u32,
    #[serde(default)]
    pub crit: u32,
    #[serde(default)]
    pub weapon_animation: u32,
}

/// A skill (spell/ability) definition, read by the skill menu and battle
/// system: its 1-based id, name, description, `sp_cost` (SP spent to cast),
/// `power` (base effect magnitude), and `hit` (base success rate, percent).
///
/// The remaining fields carry the RM2000 battle effect. `scope` picks its
/// targets (`0` one enemy, `1` all enemies, `2` self, `3` one ally, `4` all
/// allies) and `skill_type` its family (`0` normal — the only battle-relevant
/// kind — `1` teleport, `2` escape, `3` switch). `physical_rate`/`magical_rate`
/// (0–10) weight the caster's attack versus spirit in the damage formula.
/// `affect_hp`/`affect_sp` mark which pool the effect changes and `absorb`
/// whether the caster drains what it deals. `attributes` holds the 1-based
/// element ids the damage is checked against and `affected_states` the 1-based
/// state ids the skill inflicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDef {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub sp_cost: u32,
    pub power: u32,
    pub hit: u32,
    #[serde(default)]
    pub skill_type: u32,
    #[serde(default)]
    pub scope: u32,
    #[serde(default)]
    pub physical_rate: u32,
    #[serde(default)]
    pub magical_rate: u32,
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
/// (0–100) decides which active state's graphic and restriction dominate.
/// Recovery is governed by `hold_turn` (minimum turns held before it can lift),
/// `auto_release_prob` (percent chance per turn to lift afterwards), and
/// `release_by_damage` (percent chance to lift when hit by a physical attack).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateDef {
    pub id: u32,
    pub name: String,
    pub restriction: u32,
    pub priority: u32,
    pub hold_turn: u32,
    pub auto_release_prob: u32,
    pub release_by_damage: u32,
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

/// One entry in a monster's battle-AI list (see `lcf::EnemyAction`): the action
/// family (`kind`/`basic`), the `skill_id`/`enemy_id` it targets, the condition
/// gating it (`condition_type` + `condition_min`/`condition_max`), and its
/// `priority` for tie-breaking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyActionDef {
    pub kind: u32,
    pub basic: u32,
    pub skill_id: u32,
    pub enemy_id: u32,
    pub condition_type: u32,
    pub condition_min: u32,
    pub condition_max: u32,
    pub priority: u32,
}

/// One member of a troop: the `enemy_id` of the monster (matching a
/// [`MonsterDef::id`]) and its `x`,`y` placement on the battle backdrop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TroopMemberDef {
    pub enemy_id: u32,
    pub x: u32,
    pub y: u32,
}

/// A troop (enemy party) definition, read by the battle system to build an
/// encounter: its 1-based id, name, and the monsters it fields with their
/// positions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TroopDef {
    pub id: u32,
    pub name: String,
    pub members: Vec<TroopMemberDef>,
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
/// `lcf::AnimationCell`): `cell_id` selects the tile from the animation's
/// graphic, `x`/`y` offset it from the anchor in screen pixels, `scale` is a
/// zoom percent (`100` = full size), the four `tone_*` channels tint it on
/// RM2000's `0..=200` scale (`100` = neutral), and `transparency` is a
/// `0..=100` percent (`0` = opaque).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationCellDef {
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

/// A frame-timed flash and sound effect on an animation's timeline (see
/// `lcf::AnimationTiming`): `frame` is the 1-based frame it fires on, `se_name`
/// the sound-effect file under `audio/Sound/` (empty = silent), `flash_scope`
/// selects what flashes (`0` nothing, `1` the target, `2` the whole screen),
/// `flash_red`/`flash_green`/`flash_blue` the flash colour on RM2000's `0..=31`
/// scale, and `flash_power` its strength.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationTimingDef {
    pub frame: u32,
    pub se_name: String,
    pub flash_scope: u32,
    pub flash_red: u32,
    pub flash_green: u32,
    pub flash_blue: u32,
    pub flash_power: u32,
}
