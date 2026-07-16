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
}

/// One event command: RM2000 opcode, nesting indent, string, and int params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventCommand {
    pub code: u32,
    pub indent: u32,
    pub string: String,
    pub params: Vec<i32>,
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
