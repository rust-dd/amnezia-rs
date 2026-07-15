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

    let mut chipset_id = 0;
    let mut width = DEFAULT_WIDTH;
    let mut height = DEFAULT_HEIGHT;
    let mut lower: Option<&[u8]> = None;
    let mut upper: Option<&[u8]> = None;

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
            _ => {}
        }
    }

    let expected = width
        .checked_mul(height)
        .ok_or(LcfError::InvalidDimensions { width, height })? as usize;
    let lower_layer = decode_layer(lower, "lower", expected)?;
    let upper_layer = decode_layer(upper, "upper", expected)?;

    Ok(MapUnit { chipset_id, width, height, lower_layer, upper_layer })
}

/// A chipset entry from the database: its 1-based id and the base name of its
/// `ChipSet/<name>` graphic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chipset {
    pub id: u32,
    pub name: String,
}

const CHIPSET_SECTION: u32 = 0x14;
const CHIPSET_NAME: u32 = 0x02;

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
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            if sub_id == CHIPSET_NAME {
                name = String::from_utf8_lossy(sub_data).into_owned();
            }
        }
        chipsets.push(Chipset { id, name });
    }
    Ok(chipsets)
}

#[cfg(test)]
mod tests {
    use crate::{parse_map, parse_chipsets, Chipset, LcfError};

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
                (0x51, vec![0xDE, 0xAD, 0xBE, 0xEF]),
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
        assert_eq!(
            chipsets,
            vec![
                Chipset { id: 1, name: "basis".to_string() },
                Chipset { id: 2, name: "outline".to_string() },
            ]
        );
    }

    #[test]
    fn chipset_name_defaults_to_empty_when_omitted() {
        let element = chipset_element(5, &[]);
        let ldb = make_ldb(&[(0x14, chipset_section(&[element]))]);
        let chipsets = parse_chipsets(&ldb).unwrap();
        assert_eq!(chipsets, vec![Chipset { id: 5, name: String::new() }]);
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
    fn parse_chipsets_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x0B, vec![1, 2, 3])]);
        assert!(matches!(parse_chipsets(&ldb), Err(LcfError::MissingChipsets)));
    }
}
