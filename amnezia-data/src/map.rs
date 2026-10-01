use crate::MoveRouteDef;
use serde::{Deserialize, Serialize};

/// Tile layers contain `width * height` row-major IDs: lower ground, upper overlay.
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub name: String,
    pub pages: Vec<EventPage>,
}

/// Event page with legacy RON defaults for graphic and movement settings.
/// `direction` is the CharSet row (up=0, right=1, down=2, left=3); `pattern` is its column.
/// `move_type`: 0 stationary, 1 random, 2 vertical, 3 horizontal, 4 toward hero,
/// 5 away, 6 custom route. Frequency uses 1–8; speed uses 1–6.
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
    /// Custom route for `move_type == 6`; empty otherwise.
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

/// Map-independent script; `trigger`: 3 = autostart, 4 = parallel, 5 = call.
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
