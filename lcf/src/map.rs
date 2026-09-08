//! Map-unit (`MapXXXX.lmu`) parsing: the geometry, tile layers, and events. The
//! map-tree (`RPG_RT.lmt`) — the party start and the map-info music tree — is
//! parsed by the sibling `map_tree` module.

use crate::LcfError;
use crate::Reader;

mod events;
mod move_route;
mod panorama;
pub use panorama::Panorama;

pub(crate) use events::parse_commands;
pub use events::{Event, EventCommand, EventCondition, EventPage};
pub use move_route::{MoveCommand, MoveRoute};

const DEFAULT_WIDTH: u32 = 20;
const DEFAULT_HEIGHT: u32 = 15;

/// A parsed RPG Maker 2000 map unit (only the fields the renderer needs).
pub struct MapUnit {
    pub panorama: Option<Panorama>,
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
    let mut panorama_enabled = false;
    let mut panorama = Panorama::default();

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
            0x1F => panorama_enabled = Reader::new(data).varint()? != 0,
            0x20 => panorama.name = crate::decode_cp1250(data),
            0x21 => panorama.loop_x = Reader::new(data).varint()? != 0,
            0x22 => panorama.loop_y = Reader::new(data).varint()? != 0,
            0x23 => panorama.auto_x = Reader::new(data).varint()? != 0,
            0x24 => panorama.speed_x = Reader::new(data).varint()? as i32,
            0x25 => panorama.auto_y = Reader::new(data).varint()? != 0,
            0x26 => panorama.speed_y = Reader::new(data).varint()? as i32,
            0x47 => lower = Some(data),
            0x48 => upper = Some(data),
            0x51 => events = events::parse_events(data)?,
            _ => {}
        }
    }

    let expected = width
        .checked_mul(height)
        .ok_or(LcfError::InvalidDimensions { width, height })? as usize;
    let lower_layer = decode_layer(lower, "lower", expected)?;
    let upper_layer = decode_layer(upper, "upper", expected)?;

    Ok(MapUnit {
        panorama: panorama_enabled.then_some(panorama),
        chipset_id,
        width,
        height,
        lower_layer,
        upper_layer,
        events,
    })
}

#[cfg(test)]
mod tests {
    use crate::test_util::{subchunk, varint};
    use crate::{LcfError, parse_map};

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
    fn reads_panorama_switches_and_signed_scroll_speed() {
        let file = make_lmu(
            b"LcfMapUnit",
            &[
                (0x02, varint(1)),
                (0x03, varint(1)),
                (0x1F, varint(1)),
                (0x20, b"Sky".to_vec()),
                (0x21, varint(1)),
                (0x23, varint(1)),
                (0x24, varint((-3_i32) as u32)),
                (0x47, layer_bytes(&[0])),
                (0x48, layer_bytes(&[10000])),
            ],
        );
        let panorama = parse_map(&file).unwrap().panorama.unwrap();
        assert_eq!(panorama.name, "Sky");
        assert!(panorama.loop_x && panorama.auto_x);
        assert_eq!(panorama.speed_x, -3);
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
    fn parses_move_fields_and_applies_defaults() {
        // Page 1 carries explicit move chunks (0x1F type 2, 0x20 freq 6, 0x25
        // speed 5); page 2 carries none, so move_type defaults to 1 (random) and
        // frequency/speed to 3 — the RM2000 defaults an omitted chunk stands for.
        let mut page_a = varint(1);
        page_a.extend(subchunk(0x1F, &varint(2)));
        page_a.extend(subchunk(0x20, &varint(6)));
        page_a.extend(subchunk(0x25, &varint(5)));
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
        assert_eq!(
            (
                pages[0].move_type,
                pages[0].move_frequency,
                pages[0].move_speed
            ),
            (2, 6, 5)
        );
        assert_eq!(
            (
                pages[1].move_type,
                pages[1].move_frequency,
                pages[1].move_speed
            ),
            (1, 3, 3)
        );
    }

    #[test]
    fn parses_page_move_route() {
        // A move_type-6 page carrying a route chunk (0x29): commands move-down (2)
        // and change_graphic "Torch" frame 1, with the repeat flag set.
        let cmds = {
            let mut c = varint(2);
            c.extend(varint(34));
            c.extend(varint(5));
            c.extend_from_slice(b"Torch");
            c.extend(varint(1));
            c
        };
        let route = {
            let mut r = subchunk(0x0C, &cmds);
            r.extend(subchunk(0x15, &varint(1)));
            r.push(0);
            r
        };
        let mut page = varint(1);
        page.extend(subchunk(0x1F, &varint(6)));
        page.extend(subchunk(0x29, &route));
        page.push(0);
        let mut pages = varint(1);
        pages.extend_from_slice(&page);
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
        let page = &map.events[0].pages[0];
        assert_eq!(page.move_type, 6);
        assert!(page.move_route.repeat);
        assert_eq!(page.move_route.commands.len(), 2);
        assert_eq!(page.move_route.commands[0].code, 2);
        assert_eq!(page.move_route.commands[1].code, 34);
        assert_eq!(page.move_route.commands[1].string, "Torch");
        assert_eq!(page.move_route.commands[1].params, vec![1]);
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
