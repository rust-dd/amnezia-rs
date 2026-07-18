//! Map-unit (`MapXXXX.lmu`) and map-tree (`RPG_RT.lmt`) parsing: the geometry,
//! tile layers, events, and the party's starting position.

use crate::LcfError;
use crate::{Reader, decode_cp1250};

const DEFAULT_WIDTH: u32 = 20;
const DEFAULT_HEIGHT: u32 = 15;

/// A parsed RPG Maker 2000 map unit (only the fields the renderer needs).
pub struct MapUnit {
    pub chipset_id: u32,
    pub width: u32,
    pub height: u32,
    pub lower_layer: Vec<u16>,
    pub upper_layer: Vec<u16>,
    pub events: Vec<Event>,
}

fn decode_layer(
    data: Option<&[u8]>,
    layer: &'static str,
    expected: usize,
) -> Result<Vec<u16>, LcfError> {
    let data = data.unwrap_or(&[]);
    if data.len() != expected * 2 {
        return Err(LcfError::LayerSizeMismatch {
            layer,
            got: data.len() / 2,
            expected,
        });
    }
    Ok(data
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect())
}

/// Parse an LMU (`MapXXXX.lmu`) byte slice into a [`MapUnit`], applying
/// RPG Maker 2000 defaults for any omitted fields.
pub fn parse_map(bytes: &[u8]) -> Result<MapUnit, LcfError> {
    let mut reader = Reader::new(bytes);
    let signature_len = reader.byte()? as usize;
    let signature = reader.take(signature_len)?;
    if signature != b"LcfMapUnit" {
        return Err(LcfError::BadSignature {
            expected: "LcfMapUnit",
        });
    }

    let mut chipset_id = 1;
    let mut width = DEFAULT_WIDTH;
    let mut height = DEFAULT_HEIGHT;
    let mut lower: Option<&[u8]> = None;
    let mut upper: Option<&[u8]> = None;
    let mut events = Vec::new();

    while !reader.is_empty() {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let data = reader.take(size)?;
        match id {
            0x01 => chipset_id = Reader::new(data).varint()?,
            0x02 => width = Reader::new(data).varint()?,
            0x03 => height = Reader::new(data).varint()?,
            0x47 => lower = Some(data),
            0x48 => upper = Some(data),
            0x51 => events = parse_events(data)?,
            _ => {}
        }
    }

    let expected = width
        .checked_mul(height)
        .ok_or(LcfError::InvalidDimensions { width, height })? as usize;
    let lower_layer = decode_layer(lower, "lower", expected)?;
    let upper_layer = decode_layer(upper, "upper", expected)?;

    Ok(MapUnit {
        chipset_id,
        width,
        height,
        lower_layer,
        upper_layer,
        events,
    })
}

/// The starting party position from the map tree (`RPG_RT.lmt`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    pub map_id: u32,
    pub x: u32,
    pub y: u32,
}

/// Skip a chunk stream (`[id][size][data]*` terminated by id 0).
fn skip_chunk_stream(reader: &mut Reader) -> Result<(), LcfError> {
    loop {
        let id = reader.varint()?;
        if id == 0 {
            return Ok(());
        }
        let size = reader.varint()? as usize;
        reader.take(size)?;
    }
}

/// Parse the starting party position out of an LMT (`RPG_RT.lmt`). The tree
/// begins with the map-info list (`[count]` then per entry a bare map id + a
/// chunk stream), then the tree order (`[count]` + ids), the active node, and
/// finally the `Start` struct (`party_map_id` 0x01, `party_x` 0x02,
/// `party_y` 0x03; omitted fields default to 0).
pub fn parse_start(bytes: &[u8]) -> Result<Start, LcfError> {
    let mut reader = Reader::new(bytes);
    let signature_len = reader.byte()? as usize;
    let signature = reader.take(signature_len)?;
    if signature != b"LcfMapTree" {
        return Err(LcfError::BadSignature {
            expected: "LcfMapTree",
        });
    }

    let map_count = reader.varint()?;
    for _ in 0..map_count {
        let _map_id = reader.varint()?;
        skip_chunk_stream(&mut reader)?;
    }
    let order_count = reader.varint()?;
    for _ in 0..order_count {
        reader.varint()?;
    }
    let _active_node = reader.varint()?;

    let mut start = Start {
        map_id: 0,
        x: 0,
        y: 0,
    };
    loop {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let data = reader.take(size)?;
        match id {
            0x01 => start.map_id = Reader::new(data).varint()?,
            0x02 => start.x = Reader::new(data).varint()?,
            0x03 => start.y = Reader::new(data).varint()?,
            _ => {}
        }
    }
    Ok(start)
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventPage {
    pub trigger: u32,
    pub graphic_name: String,
    pub graphic_index: u32,
    pub direction: u32,
    pub pattern: u32,
    pub layer: u32,
    pub condition: EventCondition,
    pub commands: Vec<EventCommand>,
}

/// A page's activation condition. `flags` bits: 0 switch_a, 1 switch_b,
/// 2 variable, 3 item, 4 actor, 5 timer. A page is active when every enabled
/// flag's condition holds; `flags == 0` is always active.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventCondition {
    pub flags: u32,
    pub switch_a: u32,
    pub switch_b: u32,
    pub variable_id: u32,
    pub variable_value: u32,
    pub item_id: u32,
    pub actor_id: u32,
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

fn parse_events(data: &[u8]) -> Result<Vec<Event>, LcfError> {
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
                0x21 => page.trigger = Reader::new(sub_data).varint()?,
                0x22 => page.layer = Reader::new(sub_data).varint()?,
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
/// until the buffer is exhausted. Shared by map event pages and common events.
pub(crate) fn parse_commands(data: &[u8]) -> Result<Vec<EventCommand>, LcfError> {
    let mut reader = Reader::new(data);
    let mut commands = Vec::new();
    while !reader.is_empty() {
        let code = reader.varint()?;
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

#[cfg(test)]
mod tests {
    use crate::test_util::{subchunk, varint};
    use crate::{LcfError, Start, parse_map, parse_start};

    fn layer_bytes(tiles: &[u16]) -> Vec<u8> {
        tiles.iter().flat_map(|t| t.to_le_bytes()).collect()
    }

    fn make_lmu(signature: &[u8], chunks: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut out = vec![signature.len() as u8];
        out.extend_from_slice(signature);
        for (id, data) in chunks {
            out.extend(varint(*id));
            out.extend(varint(data.len() as u32));
            out.extend_from_slice(data);
        }
        out.push(0);
        out
    }

    #[test]
    fn parses_all_fields() {
        let file = make_lmu(
            b"LcfMapUnit",
            &[
                (0x01, varint(7)),
                (0x02, varint(2)),
                (0x03, varint(1)),
                (0x47, layer_bytes(&[1, 2])),
                (0x48, layer_bytes(&[10, 11])),
            ],
        );
        let map = parse_map(&file).unwrap();
        assert_eq!(map.chipset_id, 7);
        assert_eq!((map.width, map.height), (2, 1));
        assert_eq!(map.lower_layer, vec![1, 2]);
        assert_eq!(map.upper_layer, vec![10, 11]);
    }

    #[test]
    fn applies_defaults_for_omitted_width_height() {
        let file = make_lmu(
            b"LcfMapUnit",
            &[
                (0x01, varint(3)),
                (0x47, layer_bytes(&vec![0u16; 300])),
                (0x48, layer_bytes(&vec![0u16; 300])),
            ],
        );
        let map = parse_map(&file).unwrap();
        assert_eq!((map.width, map.height), (20, 15));
        assert_eq!(map.lower_layer.len(), 300);
        assert_eq!(map.upper_layer.len(), 300);
    }

    #[test]
    fn defaults_absent_chipset_to_one() {
        // RM2000 omits a field equal to its default; the map chipset default is
        // 1 (chipset ids are 1-based), so an LMU with no `0x01` chunk means
        // chipset 1 — not 0, which is no chipset at all.
        let file = make_lmu(
            b"LcfMapUnit",
            &[
                (0x02, varint(2)),
                (0x03, varint(1)),
                (0x47, layer_bytes(&[0, 0])),
                (0x48, layer_bytes(&[0, 0])),
            ],
        );
        let map = parse_map(&file).unwrap();
        assert_eq!(map.chipset_id, 1);
    }

    #[test]
    fn rejects_bad_signature() {
        let file = make_lmu(b"LcfMapTree", &[]);
        assert!(matches!(
            parse_map(&file),
            Err(LcfError::BadSignature {
                expected: "LcfMapUnit"
            })
        ));
    }

    #[test]
    fn rejects_layer_size_mismatch() {
        let file = make_lmu(
            b"LcfMapUnit",
            &[
                (0x02, varint(2)),
                (0x03, varint(2)),
                (0x47, layer_bytes(&[1])),
            ],
        );
        assert!(matches!(
            parse_map(&file),
            Err(LcfError::LayerSizeMismatch {
                layer: "lower",
                got: 1,
                expected: 4
            })
        ));
    }

    #[test]
    fn decodes_multibyte_scalar_value() {
        let file = make_lmu(
            b"LcfMapUnit",
            &[
                (0x01, varint(200)),
                (0x02, varint(2)),
                (0x03, varint(1)),
                (0x47, layer_bytes(&[1, 2])),
                (0x48, layer_bytes(&[3, 4])),
            ],
        );
        let map = parse_map(&file).unwrap();
        assert_eq!(map.chipset_id, 200);
    }

    #[test]
    fn rejects_overflowing_dimensions() {
        let file = make_lmu(
            b"LcfMapUnit",
            &[(0x02, varint(100_000)), (0x03, varint(100_000))],
        );
        assert!(matches!(
            parse_map(&file),
            Err(LcfError::InvalidDimensions {
                width: 100_000,
                height: 100_000
            })
        ));
    }

    #[test]
    fn skips_unknown_chunks() {
        let file = make_lmu(
            b"LcfMapUnit",
            &[
                (0x01, varint(5)),
                (0x02, varint(2)),
                (0x03, varint(1)),
                (0x0B, varint(0)),
                (0x47, layer_bytes(&[1, 2])),
                (0x63, vec![0xDE, 0xAD, 0xBE, 0xEF]),
                (0x48, layer_bytes(&[3, 4])),
            ],
        );
        let map = parse_map(&file).unwrap();
        assert_eq!(map.chipset_id, 5);
        assert_eq!((map.width, map.height), (2, 1));
        assert_eq!(map.lower_layer, vec![1, 2]);
        assert_eq!(map.upper_layer, vec![3, 4]);
    }

    #[test]
    fn rejects_truncated_input() {
        let mut file = vec![10u8];
        file.extend_from_slice(b"LcfMapUnit");
        file.push(0x47);
        file.extend(varint(999));
        file.extend([0x00, 0x00]);
        assert!(matches!(parse_map(&file), Err(LcfError::UnexpectedEof)));
    }

    #[test]
    fn parses_page_condition() {
        let cond = {
            let mut c = subchunk(0x01, &varint(1));
            c.extend(subchunk(0x02, &varint(2)));
            c.extend(subchunk(0x06, &varint(5)));
            c.extend(subchunk(0x07, &varint(3)));
            c.push(0);
            c
        };
        let mut page = varint(1);
        page.extend(subchunk(0x02, &cond));
        page.extend(subchunk(0x21, &varint(1)));
        page.extend(varint(0));
        let mut pages = varint(1);
        pages.extend_from_slice(&page);
        let mut event = varint(7);
        event.extend(subchunk(0x05, &pages));
        event.extend(varint(0));
        let mut section = varint(1);
        section.extend_from_slice(&event);
        let file = make_lmu(
            b"LcfMapUnit",
            &[
                (0x02, varint(2)),
                (0x03, varint(1)),
                (0x47, layer_bytes(&[0, 0])),
                (0x48, layer_bytes(&[0, 0])),
                (0x51, section),
            ],
        );
        let map = parse_map(&file).unwrap();
        let condition = &map.events[0].pages[0].condition;
        assert_eq!((condition.flags, condition.switch_a), (1, 2));
        assert_eq!((condition.item_id, condition.actor_id), (5, 3));
    }

    #[test]
    fn parses_page_direction_and_defaults_pattern() {
        // Page 1 carries an explicit direction chunk (0x17 = 3, left) but no
        // pattern chunk; page 2 carries neither. The explicit direction survives,
        // an omitted pattern defaults to 1, and an omitted direction to 2 (down).
        let mut page_a = varint(1);
        page_a.extend(subchunk(0x17, &varint(3)));
        page_a.push(0);
        let mut page_b = varint(2);
        page_b.push(0);
        let mut pages = varint(2);
        pages.extend_from_slice(&page_a);
        pages.extend_from_slice(&page_b);
        let mut event = varint(7);
        event.extend(subchunk(0x05, &pages));
        event.push(0);
        let mut section = varint(1);
        section.extend_from_slice(&event);
        let file = make_lmu(
            b"LcfMapUnit",
            &[
                (0x02, varint(2)),
                (0x03, varint(1)),
                (0x47, layer_bytes(&[0, 0])),
                (0x48, layer_bytes(&[0, 0])),
                (0x51, section),
            ],
        );
        let map = parse_map(&file).unwrap();
        let pages = &map.events[0].pages;
        assert_eq!((pages[0].direction, pages[0].pattern), (3, 1));
        assert_eq!((pages[1].direction, pages[1].pattern), (2, 1));
    }

    #[test]
    fn parses_party_start() {
        let mut body = varint(1); // map-info count
        body.extend(varint(1)); // map id
        body.extend(varint(0)); // its chunk-stream terminator
        body.extend(varint(1)); // tree-order count
        body.extend(varint(1)); // node id
        body.extend(varint(0)); // active node
        body.extend(subchunk(0x01, &varint(5))); // party_map_id = 5
        body.extend(varint(0)); // Start struct terminator
        let signature = b"LcfMapTree";
        let mut file = vec![signature.len() as u8];
        file.extend_from_slice(signature);
        file.extend_from_slice(&body);
        assert_eq!(
            parse_start(&file).unwrap(),
            Start {
                map_id: 5,
                x: 0,
                y: 0
            }
        );
    }

    #[test]
    fn parses_event_dialogue() {
        fn cmd(code: u32, s: &[u8]) -> Vec<u8> {
            let mut o = varint(code);
            o.extend(varint(0));
            o.extend(varint(s.len() as u32));
            o.extend_from_slice(s);
            o.extend(varint(0));
            o
        }
        let mut cmds = cmd(10110, b"Hello");
        cmds.extend(cmd(0, b""));
        let mut page = varint(1);
        page.extend(subchunk(0x21, &varint(0)));
        page.extend(subchunk(0x15, b"Object1"));
        page.extend(subchunk(0x34, &cmds));
        page.extend(varint(0));
        let mut pages = varint(1);
        pages.extend_from_slice(&page);
        let mut event = varint(7);
        event.extend(subchunk(0x01, b"Ron"));
        event.extend(subchunk(0x02, &varint(3)));
        event.extend(subchunk(0x03, &varint(4)));
        event.extend(subchunk(0x05, &pages));
        event.extend(varint(0));
        let mut section = varint(1);
        section.extend_from_slice(&event);
        let file = make_lmu(
            b"LcfMapUnit",
            &[
                (0x02, varint(2)),
                (0x03, varint(1)),
                (0x47, layer_bytes(&[0, 0])),
                (0x48, layer_bytes(&[0, 0])),
                (0x51, section),
            ],
        );
        let map = parse_map(&file).unwrap();
        assert_eq!(map.events.len(), 1);
        let event = &map.events[0];
        assert_eq!((event.id, event.x, event.y), (7, 3, 4));
        assert_eq!(event.name, "Ron");
        assert_eq!(event.pages[0].trigger, 0);
        assert_eq!(event.pages[0].graphic_name, "Object1");
        assert_eq!(event.pages[0].commands[0].code, 10110);
        assert_eq!(event.pages[0].commands[0].string, "Hello");
    }
}
