//! Parser for RPG Maker 2000 LCF binary files.
//!
//! LCF files begin with a length-prefixed ASCII signature, then a sequence of
//! `[varint id][varint size][data]` chunks terminated by a chunk with id 0.
//! Integers use a base-128 big-endian varint (high bit = continuation). Fields
//! equal to their RPG Maker 2000 default are omitted, so the parser supplies
//! defaults. This crate is dev-time tooling for the asset converter and is
//! never linked into the shipped game binary.
//!
//! Parsing is split by file: [`map`] handles map units and the map tree, and
//! [`database`] handles the `RPG_RT.ldb` chipset, actor, skill, item, monster,
//! and troop tables.

mod database;
mod map;

pub use database::{
    parse_actors, parse_chipsets, parse_items, parse_monsters, parse_skills, parse_troops, Actor,
    Chipset, Item, Monster, Skill, Troop, TroopMember,
};
pub use map::{
    parse_map, parse_start, Event, EventCommand, EventCondition, EventPage, MapUnit, Start,
};

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
    #[error("LCF database has no skill section (chunk 0x0C)")]
    MissingSkills,
    #[error("LCF database has no item section (chunk 0x0D)")]
    MissingItems,
    #[error("LCF database has no enemy section (chunk 0x0E)")]
    MissingMonsters,
    #[error("LCF database has no troop section (chunk 0x0F)")]
    MissingTroops,
}

/// A cursor over an LCF byte stream, shared by every parser in the crate.
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    pub(crate) fn byte(&mut self) -> Result<u8, LcfError> {
        let b = *self.data.get(self.pos).ok_or(LcfError::UnexpectedEof)?;
        self.pos += 1;
        Ok(b)
    }

    pub(crate) fn varint(&mut self) -> Result<u32, LcfError> {
        let mut value: u32 = 0;
        loop {
            let b = self.byte()?;
            value = (value << 7) | u32::from(b & 0x7F);
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], LcfError> {
        let end = self.pos.checked_add(n).ok_or(LcfError::UnexpectedEof)?;
        let slice = self.data.get(self.pos..end).ok_or(LcfError::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }
}

/// Decode a Windows-1250 (Central European) byte string, the encoding the
/// Hungarian original stores its names and dialogue in.
pub(crate) fn decode_cp1250(bytes: &[u8]) -> String {
    encoding_rs::WINDOWS_1250.decode(bytes).0.into_owned()
}

#[cfg(test)]
pub(crate) mod test_util {
    /// Encode a value as a base-128 big-endian varint (the LCF integer format).
    pub(crate) fn varint(mut v: u32) -> Vec<u8> {
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

    /// Build one `[id][size][data]` chunk.
    pub(crate) fn subchunk(id: u32, data: &[u8]) -> Vec<u8> {
        let mut out = varint(id);
        out.extend(varint(data.len() as u32));
        out.extend_from_slice(data);
        out
    }

    /// Build one struct-list entry: a 1-based `[id]`, its sub-chunks, then a
    /// terminating id 0.
    pub(crate) fn element(id: u32, subchunks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = varint(id);
        for chunk in subchunks {
            out.extend_from_slice(chunk);
        }
        out.extend(varint(0));
        out
    }

    /// Wrap struct-list entries with their `[count]` header.
    pub(crate) fn section(elements: &[Vec<u8>]) -> Vec<u8> {
        let mut out = varint(elements.len() as u32);
        for e in elements {
            out.extend_from_slice(e);
        }
        out
    }

    /// Build a minimal LDB: the `LcfDataBase` signature then each top-level
    /// `[id][size][data]` chunk and a terminating id 0.
    pub(crate) fn make_ldb(chunks: &[(u32, Vec<u8>)]) -> Vec<u8> {
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
}
