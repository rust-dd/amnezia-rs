//! The event layer of a map unit: the [`Event`] list (chunk 0x51), each event's
//! [`EventPage`]s with their trigger, graphic, autonomous-movement fields, custom
//! [`MoveRoute`](super::MoveRoute), activation [`EventCondition`], and flat
//! [`EventCommand`] script. The map geometry that wraps these lives in the parent
//! module; the flat command decoder [`parse_commands`] is shared with common
//! events in the database.

use super::move_route::{self, MoveRoute};
use crate::{LcfError, Reader, decode_cp1250};

#[cfg(test)]
mod tests;

/// A map event: its id, tile position, name, and pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub name: String,
    pub pages: Vec<EventPage>,
}

/// One page of an event: its trigger, graphic, layer, condition, and commands.
/// The layer (0 = below hero, 1 = same as hero, 2 = above hero) decides
/// collision: a `layer == 1` page blocks the player. `direction` is the CharSet
/// facing row (Up=0, Right=1, Down=2, Left=3; default 2 = down) and `pattern`
/// the walk frame column (default 1 = the standing middle frame).
///
/// `move_type` is the page's autonomous movement (0 stationary, 1 random,
/// 2 vertical pace, 3 horizontal pace, 4 toward hero, 5 away from hero, 6 custom
/// route); RM2000 always writes it, so its default only guards a malformed page
/// (liblcf's default is 1). `move_frequency` (1–8, default 3) sets how often the
/// event steps and `move_speed` (1–6, default 3) how fast each step moves; both
/// are omitted from the file when equal to their default. `move_route` is the
/// custom route a `move_type == 6` page follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventPage {
    pub trigger: u32,
    pub graphic_name: String,
    pub graphic_index: u32,
    pub direction: u32,
    pub pattern: u32,
    pub animation_type: u32,
    pub translucent: bool,
    pub overlap_forbidden: bool,
    pub move_type: u32,
    pub move_frequency: u32,
    pub move_speed: u32,
    pub move_route: MoveRoute,
    pub layer: u32,
    pub condition: EventCondition,
    pub commands: Vec<EventCommand>,
}

/// A page's activation condition. `flags` bits: 0 switch_a, 1 switch_b,
/// 2 variable, 3 item, 4 actor, 5 timer. A page is active when every enabled
/// flag's condition holds; `flags == 0` is always active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCondition {
    pub flags: u32,
    pub switch_a: u32,
    pub switch_b: u32,
    pub variable_id: u32,
    pub variable_value: u32,
    pub item_id: u32,
    pub actor_id: u32,
}

impl Default for EventCondition {
    fn default() -> Self {
        Self {
            flags: 0,
            switch_a: 1,
            switch_b: 1,
            variable_id: 1,
            variable_value: 0,
            item_id: 1,
            actor_id: 1,
        }
    }
}

fn parse_condition(data: &[u8]) -> Result<EventCondition, LcfError> {
    let mut reader = Reader::new(data);
    let mut condition = EventCondition::default();
    loop {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let field = reader.take(size)?;
        let value = Reader::new(field).varint().unwrap_or(0);
        match id {
            0x01 => condition.flags = value,
            0x02 => condition.switch_a = value,
            0x03 => condition.switch_b = value,
            0x04 => condition.variable_id = value,
            0x05 => condition.variable_value = value,
            0x06 => condition.item_id = value,
            0x07 => condition.actor_id = value,
            _ => {}
        }
    }
    Ok(condition)
}

/// One event command: RM2000 opcode, nesting indent, string, and int params.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCommand {
    pub code: u32,
    pub indent: u32,
    pub string: String,
    pub params: Vec<i32>,
}

/// Parse the map's event section (chunk 0x51): a `[count]` header then each
/// event's id and its name/x/y/pages sub-chunks.
pub(super) fn parse_events(data: &[u8]) -> Result<Vec<Event>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut events = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut event = Event {
            id,
            x: 0,
            y: 0,
            name: String::new(),
            pages: Vec::new(),
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                0x01 => event.name = decode_cp1250(sub_data),
                0x02 => event.x = Reader::new(sub_data).varint()?,
                0x03 => event.y = Reader::new(sub_data).varint()?,
                0x05 => event.pages = parse_pages(sub_data)?,
                _ => {}
            }
        }
        events.push(event);
    }
    Ok(events)
}

fn parse_pages(data: &[u8]) -> Result<Vec<EventPage>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut pages = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _page_id = reader.varint()?;
        let mut page = EventPage {
            trigger: 0,
            graphic_name: String::new(),
            graphic_index: 0,
            direction: 2,
            pattern: 1,
            animation_type: 0,
            translucent: false,
            overlap_forbidden: false,
            move_type: 1,
            move_frequency: 3,
            move_speed: 3,
            move_route: MoveRoute::default(),
            layer: 0,
            condition: EventCondition::default(),
            commands: Vec::new(),
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                0x02 => page.condition = parse_condition(sub_data)?,
                0x15 => page.graphic_name = decode_cp1250(sub_data),
                0x16 => page.graphic_index = Reader::new(sub_data).varint()?,
                0x17 => page.direction = Reader::new(sub_data).varint()?,
                0x18 => page.pattern = Reader::new(sub_data).varint()?,
                0x19 => page.translucent = Reader::new(sub_data).varint()? != 0,
                0x1F => page.move_type = Reader::new(sub_data).varint()?,
                0x20 => page.move_frequency = Reader::new(sub_data).varint()?,
                0x21 => page.trigger = Reader::new(sub_data).varint()?,
                0x22 => page.layer = Reader::new(sub_data).varint()?,
                0x23 => page.overlap_forbidden = Reader::new(sub_data).varint()? != 0,
                0x24 => page.animation_type = Reader::new(sub_data).varint()?,
                0x25 => page.move_speed = Reader::new(sub_data).varint()?,
                0x29 => page.move_route = move_route::parse_move_route(sub_data)?,
                0x34 => page.commands = parse_commands(sub_data)?,
                _ => {}
            }
        }
        pages.push(page);
    }
    Ok(pages)
}

/// Parse a flat event-command stream: repeated
/// `[code][indent][strlen][CP1250 string][paramcount][params]` records read
/// until the four-byte zero terminator or buffer end. Shared by map, common
/// and battle events.
pub(crate) fn parse_commands(data: &[u8]) -> Result<Vec<EventCommand>, LcfError> {
    let mut reader = Reader::new(data);
    let mut commands = Vec::new();
    while !reader.is_empty() {
        let code = reader.varint()?;
        if code == 0 {
            reader.take(3)?;
            break;
        }
        let indent = reader.varint()?;
        let string_len = reader.varint()? as usize;
        let string = decode_cp1250(reader.take(string_len)?);
        let param_count = reader.varint()?;
        let mut params = Vec::with_capacity(param_count as usize);
        for _ in 0..param_count {
            params.push(reader.varint()? as i32);
        }
        commands.push(EventCommand {
            code,
            indent,
            string,
            params,
        });
    }
    Ok(commands)
}
