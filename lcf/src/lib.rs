//! Parser for RPG Maker 2000 LCF binary files.
//!
//! LCF files begin with a length-prefixed ASCII signature, then a sequence of
//! `[varint id][varint size][data]` chunks terminated by a chunk with id 0.
//! Integers use a base-128 big-endian varint (high bit = continuation). Fields
//! equal to their RPG Maker 2000 default are omitted, so the parser supplies
//! defaults. This crate is dev-time tooling for the asset converter and is
//! never linked into the shipped game binary.

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

/// Errors returned while parsing an LCF file.
#[derive(Debug, thiserror::Error)]
pub enum LcfError {
    #[error("bad LCF signature: expected {expected:?}")]
    BadSignature { expected: &'static str },
    #[error("unexpected end of data")]
    UnexpectedEof,
    #[error("{layer} layer has {got} tiles but width*height = {expected}")]
    LayerSizeMismatch { layer: &'static str, got: usize, expected: usize },
    #[error("invalid map dimensions {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },
    #[error("LCF database has no chipset section (chunk 0x14)")]
    MissingChipsets,
    #[error("LCF database has no actor section (chunk 0x0B)")]
    MissingActors,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn byte(&mut self) -> Result<u8, LcfError> {
        let b = *self.data.get(self.pos).ok_or(LcfError::UnexpectedEof)?;
        self.pos += 1;
        Ok(b)
    }

    fn varint(&mut self) -> Result<u32, LcfError> {
        let mut value: u32 = 0;
        loop {
            let b = self.byte()?;
            value = (value << 7) | u32::from(b & 0x7F);
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], LcfError> {
        let end = self.pos.checked_add(n).ok_or(LcfError::UnexpectedEof)?;
        let slice = self.data.get(self.pos..end).ok_or(LcfError::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }
}

fn decode_layer(data: Option<&[u8]>, layer: &'static str, expected: usize) -> Result<Vec<u16>, LcfError> {
    let data = data.unwrap_or(&[]);
    if data.len() != expected * 2 {
        return Err(LcfError::LayerSizeMismatch { layer, got: data.len() / 2, expected });
    }
    Ok(data.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect())
}

/// Parse an LMU (`MapXXXX.lmu`) byte slice into a [`MapUnit`], applying
/// RPG Maker 2000 defaults for any omitted fields.
pub fn parse_map(bytes: &[u8]) -> Result<MapUnit, LcfError> {
    let mut reader = Reader::new(bytes);
    let signature_len = reader.byte()? as usize;
    let signature = reader.take(signature_len)?;
    if signature != b"LcfMapUnit" {
        return Err(LcfError::BadSignature { expected: "LcfMapUnit" });
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

    Ok(MapUnit { chipset_id, width, height, lower_layer, upper_layer, events })
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
        return Err(LcfError::BadSignature { expected: "LcfMapTree" });
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

    let mut start = Start { map_id: 0, x: 0, y: 0 };
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
/// collision: a `layer == 1` page blocks the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventPage {
    pub trigger: u32,
    pub graphic_name: String,
    pub graphic_index: u32,
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

fn decode_cp1250(bytes: &[u8]) -> String {
    encoding_rs::WINDOWS_1250.decode(bytes).0.into_owned()
}

fn parse_events(data: &[u8]) -> Result<Vec<Event>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut events = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut event = Event { id, x: 0, y: 0, name: String::new(), pages: Vec::new() };
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

fn parse_commands(data: &[u8]) -> Result<Vec<EventCommand>, LcfError> {
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
        commands.push(EventCommand { code, indent, string, params });
    }
    Ok(commands)
}

/// A chipset entry from the database: its 1-based id, the base name of its
/// `ChipSet/<name>` graphic, and its passability arrays (per tile-type
/// bitfields; lower is 162 bytes, upper is 144, padded with `0x0F`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chipset {
    pub id: u32,
    pub name: String,
    pub passages_down: Vec<u8>,
    pub passages_up: Vec<u8>,
}

const CHIPSET_SECTION: u32 = 0x14;
const CHIPSET_NAME: u32 = 0x02;
const CHIPSET_PASSAGES_DOWN: u32 = 0x04;
const CHIPSET_PASSAGES_UP: u32 = 0x05;
const PASSAGES_DOWN_LEN: usize = 162;
const PASSAGES_UP_LEN: usize = 144;
const PASSAGE_DEFAULT: u8 = 0x0F;

/// Parse the chipset graphic names out of an LDB (`RPG_RT.ldb`) byte slice.
/// Only the chipset section is read; every other database section is skipped.
pub fn parse_chipsets(bytes: &[u8]) -> Result<Vec<Chipset>, LcfError> {
    let mut reader = Reader::new(bytes);
    let signature_len = reader.byte()? as usize;
    let signature = reader.take(signature_len)?;
    if signature != b"LcfDataBase" {
        return Err(LcfError::BadSignature { expected: "LcfDataBase" });
    }

    let mut section: Option<&[u8]> = None;
    while !reader.is_empty() {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let data = reader.take(size)?;
        if id == CHIPSET_SECTION {
            section = Some(data);
            break;
        }
    }
    let section = section.ok_or(LcfError::MissingChipsets)?;

    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut chipsets = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut name = String::new();
        let mut passages_down: Vec<u8> = Vec::new();
        let mut passages_up: Vec<u8> = Vec::new();
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                CHIPSET_NAME => name = String::from_utf8_lossy(sub_data).into_owned(),
                CHIPSET_PASSAGES_DOWN => passages_down = sub_data.to_vec(),
                CHIPSET_PASSAGES_UP => passages_up = sub_data.to_vec(),
                _ => {}
            }
        }
        passages_down.resize(PASSAGES_DOWN_LEN, PASSAGE_DEFAULT);
        passages_up.resize(PASSAGES_UP_LEN, PASSAGE_DEFAULT);
        chipsets.push(Chipset { id, name, passages_down, passages_up });
    }
    Ok(chipsets)
}

/// An actor (playable character) entry from the database: its 1-based id and
/// its default name. Only the name is needed by the game, to expand the
/// `\N[k]` message control code that inserts an actor's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub id: u32,
    pub name: String,
}

const ACTOR_SECTION: u32 = 0x0B;
const ACTOR_NAME: u32 = 0x01;

/// Parse the actor table out of an LDB (`RPG_RT.ldb`) byte slice. The actor
/// section (chunk `0x0B`) shares the chipset section's struct-list shape: a
/// `[count]` followed by entries, each a 1-based id then a chunk stream whose
/// name sub-chunk (`0x01`) is a CP1250 string. Every other section is skipped.
pub fn parse_actors(bytes: &[u8]) -> Result<Vec<Actor>, LcfError> {
    let mut reader = Reader::new(bytes);
    let signature_len = reader.byte()? as usize;
    let signature = reader.take(signature_len)?;
    if signature != b"LcfDataBase" {
        return Err(LcfError::BadSignature { expected: "LcfDataBase" });
    }

    let mut section: Option<&[u8]> = None;
    while !reader.is_empty() {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let data = reader.take(size)?;
        if id == ACTOR_SECTION {
            section = Some(data);
            break;
        }
    }
    let section = section.ok_or(LcfError::MissingActors)?;

    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut actors = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut name = String::new();
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            if sub_id == ACTOR_NAME {
                name = decode_cp1250(sub_data);
            }
        }
        actors.push(Actor { id, name });
    }
    Ok(actors)
}

#[cfg(test)]
mod tests {
    use crate::{parse_actors, parse_chipsets, parse_map, parse_start, Actor, LcfError, Start};

    fn varint(mut v: u32) -> Vec<u8> {
        let mut groups = vec![(v & 0x7F) as u8];
        v >>= 7;
        while v != 0 {
            groups.push((v & 0x7F) as u8);
            v >>= 7;
        }
        groups.reverse();
        let last = groups.len() - 1;
        groups
            .iter()
            .enumerate()
            .map(|(i, b)| if i < last { b | 0x80 } else { *b })
            .collect()
    }

    fn layer_bytes(tiles: &[u16]) -> Vec<u8> {
        tiles.iter().flat_map(|t| t.to_le_bytes()).collect()
    }

    fn subchunk(id: u32, data: &[u8]) -> Vec<u8> {
        let mut out = varint(id);
        out.extend(varint(data.len() as u32));
        out.extend_from_slice(data);
        out
    }

    fn chipset_element(id: u32, subchunks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = varint(id);
        for chunk in subchunks {
            out.extend_from_slice(chunk);
        }
        out.extend(varint(0));
        out
    }

    fn chipset_section(elements: &[Vec<u8>]) -> Vec<u8> {
        let mut out = varint(elements.len() as u32);
        for element in elements {
            out.extend_from_slice(element);
        }
        out
    }

    fn make_ldb(chunks: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let signature = b"LcfDataBase";
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
            Err(LcfError::BadSignature { expected: "LcfMapUnit" })
        ));
    }

    #[test]
    fn rejects_layer_size_mismatch() {
        let file = make_lmu(
            b"LcfMapUnit",
            &[(0x02, varint(2)), (0x03, varint(2)), (0x47, layer_bytes(&[1]))],
        );
        assert!(matches!(
            parse_map(&file),
            Err(LcfError::LayerSizeMismatch { layer: "lower", got: 1, expected: 4 })
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
            Err(LcfError::InvalidDimensions { width: 100_000, height: 100_000 })
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
    fn parses_chipset_graphic_names() {
        let element1 = chipset_element(1, &[subchunk(0x01, b"World"), subchunk(0x02, b"basis")]);
        let element2 = chipset_element(2, &[subchunk(0x02, b"outline")]);
        let ldb = make_ldb(&[(0x0B, vec![1, 2, 3]), (0x14, chipset_section(&[element1, element2]))]);
        let chipsets = parse_chipsets(&ldb).unwrap();
        assert_eq!(chipsets.len(), 2);
        assert_eq!(chipsets[0].id, 1);
        assert_eq!(chipsets[0].name, "basis");
        assert_eq!(chipsets[1].name, "outline");
        assert_eq!(chipsets[0].passages_down.len(), 162);
        assert_eq!(chipsets[0].passages_up.len(), 144);
        assert!(chipsets[0].passages_down.iter().all(|&b| b == 0x0F));
    }

    #[test]
    fn chipset_name_defaults_to_empty_when_omitted() {
        let element = chipset_element(5, &[]);
        let ldb = make_ldb(&[(0x14, chipset_section(&[element]))]);
        let chipsets = parse_chipsets(&ldb).unwrap();
        assert_eq!(chipsets[0].id, 5);
        assert!(chipsets[0].name.is_empty());
        assert_eq!(chipsets[0].passages_down.len(), 162);
    }

    #[test]
    fn captures_and_pads_passability() {
        let element = chipset_element(
            1,
            &[subchunk(0x02, b"x"), subchunk(0x04, &[0x00, 0x0F]), subchunk(0x05, &[0x1F])],
        );
        let ldb = make_ldb(&[(0x14, chipset_section(&[element]))]);
        let chipset = &parse_chipsets(&ldb).unwrap()[0];
        assert_eq!(chipset.passages_down.len(), 162);
        assert_eq!(chipset.passages_up.len(), 144);
        assert_eq!(&chipset.passages_down[..2], &[0x00, 0x0F]);
        assert_eq!(chipset.passages_down[2], 0x0F);
        assert_eq!(chipset.passages_up[0], 0x1F);
    }

    #[test]
    fn parse_chipsets_rejects_bad_signature() {
        let ldb = {
            let mut out = vec![10u8];
            out.extend_from_slice(b"LcfMapUnit");
            out.push(0);
            out
        };
        assert!(matches!(
            parse_chipsets(&ldb),
            Err(LcfError::BadSignature { expected: "LcfDataBase" })
        ));
    }

    #[test]
    fn parses_page_condition() {
        let cond = {
            let mut c = subchunk(0x01, &varint(1));
            c.extend(subchunk(0x02, &varint(2)));
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
        assert_eq!(parse_start(&file).unwrap(), Start { map_id: 5, x: 0, y: 0 });
    }

    #[test]
    fn parse_chipsets_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x0B, vec![1, 2, 3])]);
        assert!(matches!(parse_chipsets(&ldb), Err(LcfError::MissingChipsets)));
    }

    #[test]
    fn parses_actor_names() {
        // The actor section (chunk 0x0B) is the same struct-list shape as the
        // chipset section, so the chipset element/section builders apply. The
        // hero's name bytes are CP1250: 0x41 0x64 0xE9 0x6C -> "Adél".
        let hero = chipset_element(1, &[subchunk(0x01, &[0x41, 0x64, 0xE9, 0x6C])]);
        let mage = chipset_element(2, &[subchunk(0x01, b"Bob")]);
        let ldb = make_ldb(&[(0x05, vec![9, 9]), (0x0B, chipset_section(&[hero, mage]))]);
        let actors = parse_actors(&ldb).unwrap();
        assert_eq!(actors.len(), 2);
        assert_eq!(actors[0], Actor { id: 1, name: "Adél".to_string() });
        assert_eq!(actors[1], Actor { id: 2, name: "Bob".to_string() });
    }

    #[test]
    fn actor_name_defaults_to_empty_when_omitted() {
        let actor = chipset_element(3, &[]);
        let ldb = make_ldb(&[(0x0B, chipset_section(&[actor]))]);
        let actors = parse_actors(&ldb).unwrap();
        assert_eq!(actors[0].id, 3);
        assert!(actors[0].name.is_empty());
    }

    #[test]
    fn parse_actors_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, chipset_section(&[]))]);
        assert!(matches!(parse_actors(&ldb), Err(LcfError::MissingActors)));
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
