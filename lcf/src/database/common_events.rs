//! Common-event definitions from the database (`ChunkData::common_events`,
//! `0x19`). A common event is a globally-callable event script: a trigger, an
//! optional condition switch, and a command list in the same flat format map
//! event pages use. Chunk ids follow liblcf `ChunkCommonEvent`.

use super::find_section;
use crate::map::parse_commands;
use crate::{decode_cp1250, EventCommand, LcfError, Reader};

/// A common event (global event script): its 1-based id, name, `trigger`
/// (0 = call, 1 = autostart, 2 = parallel), the `switch_id` gating an
/// autostart/parallel event, and its command list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonEvent {
    pub id: u32,
    pub name: String,
    pub trigger: u32,
    pub switch_id: u32,
    pub commands: Vec<EventCommand>,
}

const COMMON_EVENT_SECTION: u32 = 0x19;
const COMMON_EVENT_NAME: u32 = 0x01;
const COMMON_EVENT_TRIGGER: u32 = 0x0B;
const COMMON_EVENT_SWITCH_ID: u32 = 0x0D;
const COMMON_EVENT_COMMANDS: u32 = 0x16;

/// Parse the common-event table (`ChunkData::common_events` = `0x19`) out of an
/// LDB byte slice. Chunk ids (liblcf `ChunkCommonEvent`): name `0x01`, trigger
/// `0x0B`, switch_id `0x0D`, event_commands `0x16`. The command stream reuses
/// the map event-page command format ([`crate::map::parse_commands`]). Omitted
/// scalar fields default to 0; an omitted command list is empty.
pub fn parse_common_events(bytes: &[u8]) -> Result<Vec<CommonEvent>, LcfError> {
    let section = find_section(bytes, COMMON_EVENT_SECTION, LcfError::MissingCommonEvents)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut common_events = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut event =
            CommonEvent { id, name: String::new(), trigger: 0, switch_id: 0, commands: Vec::new() };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                COMMON_EVENT_NAME => event.name = decode_cp1250(sub_data),
                COMMON_EVENT_TRIGGER => event.trigger = Reader::new(sub_data).varint()?,
                COMMON_EVENT_SWITCH_ID => event.switch_id = Reader::new(sub_data).varint()?,
                COMMON_EVENT_COMMANDS => event.commands = parse_commands(sub_data)?,
                _ => {}
            }
        }
        common_events.push(event);
    }
    Ok(common_events)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{element, make_ldb, section, subchunk, varint};
    use crate::{parse_common_events, EventCommand, LcfError};

    /// Build one record of the flat command stream:
    /// `[code][indent][strlen][string][paramcount][params]`.
    fn command(code: u32, indent: u32, string: &[u8], params: &[u32]) -> Vec<u8> {
        let mut out = varint(code);
        out.extend(varint(indent));
        out.extend(varint(string.len() as u32));
        out.extend_from_slice(string);
        out.extend(varint(params.len() as u32));
        for &p in params {
            out.extend(varint(p));
        }
        out
    }

    #[test]
    fn parses_common_event_with_command_list() {
        // Message text bytes are CP1250 "Helló" (0xF3 = 'ó'); the name bytes are
        // "Kezdés" (start): 0xE9 = 'é'.
        let mut commands = command(10110, 0, &[0x48, 0x65, 0x6C, 0x6C, 0xF3], &[]);
        commands.extend(command(10, 1, b"", &[1, 2]));
        commands.extend(command(0, 0, b"", &[]));
        let intro = element(
            1,
            &[
                subchunk(0x01, &[0x4B, 0x65, 0x7A, 0x64, 0xE9, 0x73]),
                subchunk(0x0B, &varint(1)),
                subchunk(0x0D, &varint(7)),
                subchunk(0x16, &commands),
            ],
        );
        let ldb = make_ldb(&[(0x0F, section(&[])), (0x19, section(&[intro]))]);
        let events = parse_common_events(&ldb).unwrap();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.id, 1);
        assert_eq!(event.name, "Kezdés");
        assert_eq!(event.trigger, 1, "autostart");
        assert_eq!(event.switch_id, 7);
        assert_eq!(event.commands.len(), 3);
        assert_eq!(
            event.commands[0],
            EventCommand { code: 10110, indent: 0, string: "Helló".to_string(), params: vec![] }
        );
        assert_eq!(event.commands[1], EventCommand {
            code: 10,
            indent: 1,
            string: String::new(),
            params: vec![1, 2],
        });
        assert_eq!(event.commands[2].code, 0, "trailing terminator command");
    }

    #[test]
    fn common_event_defaults_when_fields_omitted() {
        let empty = element(3, &[subchunk(0x01, b"Idle")]);
        let ldb = make_ldb(&[(0x19, section(&[empty]))]);
        let events = parse_common_events(&ldb).unwrap();
        let e = &events[0];
        assert_eq!(e.id, 3);
        assert_eq!(e.name, "Idle");
        assert_eq!(e.trigger, 0, "trigger defaults to call");
        assert_eq!(e.switch_id, 0);
        assert!(e.commands.is_empty());
    }

    #[test]
    fn parse_common_events_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(parse_common_events(&ldb), Err(LcfError::MissingCommonEvents)));
    }
}
