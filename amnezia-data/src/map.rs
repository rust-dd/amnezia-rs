use crate::MoveRouteDef;
use serde::{Deserialize, Serialize};

/// A converted map: the chipset it uses, its dimensions in tiles, and the two
/// tile layers (each `width * height` tile ids, row-major). `lower` is the
/// ground layer, `upper` the overlay layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Map {
    #[serde(default, skip_serializing_if = "scrolls_neither_axis")]
    pub scroll_type: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panorama: Option<crate::PanoramaDef>,
    pub chipset_id: u32,
    pub width: u32,
    pub height: u32,
    pub lower: Vec<u16>,
    pub upper: Vec<u16>,
    pub events: Vec<Event>,
}

fn scrolls_neither_axis(value: &u32) -> bool {
    *value == 0
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

/// One page of an event: its trigger, graphic, and command list. `direction` is
/// the CharSet facing row (Up=0, Right=1, Down=2, Left=3) and `pattern` the walk
/// frame column the NPC stands at. `move_type` selects the page's autonomous
/// movement (0 stationary, 1 random, 2 vertical pace, 3 horizontal pace, 4 toward
/// hero, 5 away from hero, 6 custom route), `move_frequency` (1–8) how often it
/// steps, and `move_speed` (1–6) how fast each step tweens. Every field after the
/// graphic carries a `serde` default (direction 2 = down, pattern 1 = middle
/// frame, move_type 0 = stationary, frequency/speed 3) so map RON written before
/// these fields existed still loads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventPage {
    pub trigger: u32,
    pub graphic_name: String,
    pub graphic_index: u32,
    #[serde(default = "default_direction")]
    pub direction: u32,
    #[serde(default = "default_pattern")]
    pub pattern: u32,
    #[serde(default, skip_serializing_if = "default_animation")]
    pub animation_type: u32,
    #[serde(default, skip_serializing_if = "opaque_page")]
    pub translucent: bool,
    #[serde(default, skip_serializing_if = "allows_overlap")]
    pub overlap_forbidden: bool,
    #[serde(default)]
    pub move_type: u32,
    #[serde(default = "default_move_frequency")]
    pub move_frequency: u32,
    #[serde(default = "default_move_speed")]
    pub move_speed: u32,
    /// The custom route a `move_type == 6` page follows; empty otherwise. Carries
    /// a `serde` default so map RON written before the field existed still loads.
    #[serde(default)]
    pub move_route: MoveRouteDef,
    pub layer: u32,
    pub condition: EventCondition,
    #[serde(deserialize_with = "deserialize_commands")]
    pub commands: Vec<EventCommand>,
}

fn default_direction() -> u32 {
    2
}

fn default_pattern() -> u32 {
    1
}

fn default_animation(value: &u32) -> bool {
    *value == 0
}

fn opaque_page(translucent: &bool) -> bool {
    !*translucent
}

fn allows_overlap(forbidden: &bool) -> bool {
    !*forbidden
}

fn default_move_frequency() -> u32 {
    3
}

fn default_move_speed() -> u32 {
    3
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

pub(crate) fn deserialize_commands<'de, D>(deserializer: D) -> Result<Vec<EventCommand>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut commands = Vec::<EventCommand>::deserialize(deserializer)?;
    // Older converted assets included the LCF stream terminator as a command.
    if let Some(end) = commands.iter().position(|command| command.code == 0) {
        commands.truncate(end);
    }
    Ok(commands)
}

/// A common event (global event script), read by the interpreter: its 1-based
/// id, name, `trigger` (3 = autostart, 4 = parallel, 5 = call), an optional
/// switch condition, and its command list. Unlike a map
/// event, it belongs to no map and its commands run in the global scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonEvent {
    pub id: u32,
    pub name: String,
    pub trigger: u32,
    #[serde(default)]
    pub switch_flag: bool,
    pub switch_id: u32,
    #[serde(deserialize_with = "deserialize_commands")]
    pub commands: Vec<EventCommand>,
}

/// A chipset entry: its 1-based id (matching a map's `chipset_id`) and the
/// base name of its graphic under `graphics/ChipSet/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chipset {
    #[serde(default)]
    pub animation_type: u32,
    #[serde(default)]
    pub animation_speed: u32,
    pub id: u32,
    pub graphic: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain_data: Vec<u16>,
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
