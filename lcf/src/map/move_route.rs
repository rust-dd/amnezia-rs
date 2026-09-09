//! The RM2000 move route carried by an event page (LMU chunk 0x29) and by the
//! `MoveEvent` command: a `repeat`/`skippable` pair plus an ordered list of
//! [`MoveCommand`]s. Each command is a `code` (see `rpg::MoveCommand::Code`)
//! followed by a small, code-specific tail — one parameter for a switch toggle,
//! a file-name string and a frame for a graphic change, a name and three
//! volume/tempo/balance values for a sound effect, and nothing at all for the
//! move/turn/wait/flag commands.

use crate::{LcfError, Reader, decode_cp1250};

/// A single move-route command: its RM2000 `code`, the code-specific integer
/// parameters, and the file-name `string` a change-graphic (34) or
/// play-sound-effect (35) command carries (empty otherwise).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MoveCommand {
    pub code: u32,
    pub params: Vec<i32>,
    pub string: String,
}

/// An RM2000 move route: the ordered commands, whether the route loops
/// (`repeat`), and whether a blocked step is skipped rather than waited on
/// (`skippable`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveRoute {
    pub commands: Vec<MoveCommand>,
    pub repeat: bool,
    pub skippable: bool,
}

impl Default for MoveRoute {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            repeat: true,
            skippable: false,
        }
    }
}

/// Parse an event page's move-route sub-chunk stream (LMU chunk 0x29): sub-chunk
/// 0x0C is the packed command array, 0x15 the `repeat` flag, 0x16 `skippable`.
/// The command-count field (0x0B) is redundant — the array is read until its own
/// chunk is exhausted — so it is skipped.
pub(super) fn parse_move_route(data: &[u8]) -> Result<MoveRoute, LcfError> {
    let mut reader = Reader::new(data);
    let mut route = MoveRoute::default();
    loop {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let field = reader.take(size)?;
        match id {
            0x0C => route.commands = parse_move_commands(field)?,
            0x15 => route.repeat = Reader::new(field).varint().unwrap_or(0) != 0,
            0x16 => route.skippable = Reader::new(field).varint().unwrap_or(0) != 0,
            _ => {}
        }
    }
    Ok(route)
}

/// Parse the packed move-command array: back-to-back records, each a `code`
/// varint then a code-specific tail (every integer is a varint, as in any LCF
/// chunk). Switch on/off (32/33) carry one parameter; change-graphic (34) a
/// `[len][CP1250 name]` string then the frame index; play-sound-effect (35) the
/// name then volume, tempo, and balance. Every other code has no tail.
fn parse_move_commands(data: &[u8]) -> Result<Vec<MoveCommand>, LcfError> {
    let mut reader = Reader::new(data);
    let mut commands = Vec::new();
    while !reader.is_empty() {
        let code = reader.varint()?;
        let (params, string) = match code {
            32 | 33 => (vec![reader.varint()? as i32], String::new()),
            34 => {
                let len = reader.varint()? as usize;
                let name = decode_cp1250(reader.take(len)?);
                (vec![reader.varint()? as i32], name)
            }
            35 => {
                let len = reader.varint()? as usize;
                let name = decode_cp1250(reader.take(len)?);
                let volume = reader.varint()? as i32;
                let tempo = reader.varint()? as i32;
                let balance = reader.varint()? as i32;
                (vec![volume, tempo, balance], name)
            }
            _ => (Vec::new(), String::new()),
        };
        commands.push(MoveCommand {
            code,
            params,
            string,
        });
    }
    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::{parse_move_commands, parse_move_route};
    use crate::test_util::{subchunk, varint};

    #[test]
    fn parses_plain_move_and_turn_commands() {
        let bytes = [varint(3), varint(12), varint(23)].concat();
        let commands = parse_move_commands(&bytes).unwrap();
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].code, 3);
        assert!(commands[0].params.is_empty() && commands[0].string.is_empty());
        assert_eq!(commands[2].code, 23);
    }

    #[test]
    fn parses_switch_graphic_and_sound_tails() {
        let mut bytes = varint(32);
        bytes.extend(varint(7));
        bytes.extend(varint(34));
        bytes.extend(varint(5));
        bytes.extend_from_slice(b"Torch");
        bytes.extend(varint(1));
        bytes.extend(varint(35));
        bytes.extend(varint(4));
        bytes.extend_from_slice(b"Bird");
        bytes.extend(varint(90));
        bytes.extend(varint(100));
        bytes.extend(varint(50));
        let commands = parse_move_commands(&bytes).unwrap();
        assert_eq!(commands.len(), 3);
        assert_eq!((commands[0].code, commands[0].params[0]), (32, 7));
        assert_eq!(commands[1].code, 34);
        assert_eq!(commands[1].string, "Torch");
        assert_eq!(commands[1].params, vec![1]);
        assert_eq!(commands[2].code, 35);
        assert_eq!(commands[2].string, "Bird");
        assert_eq!(commands[2].params, vec![90, 100, 50]);
    }

    #[test]
    fn parses_route_with_flags() {
        let cmds = [varint(2), varint(2)].concat();
        let mut chunk = subchunk(0x0B, &varint(2));
        chunk.extend(subchunk(0x0C, &cmds));
        chunk.extend(subchunk(0x15, &varint(1)));
        chunk.extend(subchunk(0x16, &varint(1)));
        chunk.push(0);
        let route = parse_move_route(&chunk).unwrap();
        assert_eq!(route.commands.len(), 2);
        assert!(route.repeat && route.skippable);
    }

    #[test]
    fn defaults_to_repeating_non_skippable_when_flags_are_omitted() {
        let cmds = varint(11);
        let mut chunk = subchunk(0x0C, &cmds);
        chunk.push(0);
        let route = parse_move_route(&chunk).unwrap();
        assert_eq!(route.commands.len(), 1);
        assert!(route.repeat && !route.skippable);
    }

    #[test]
    fn preserves_explicit_non_repeating_route() {
        let mut chunk = subchunk(0x0C, &varint(11));
        chunk.extend(subchunk(0x15, &varint(0)));
        chunk.push(0);
        let route = parse_move_route(&chunk).unwrap();
        assert!(!route.repeat && !route.skippable);
    }
}
