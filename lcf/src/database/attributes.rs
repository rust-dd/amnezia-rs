//! Damage elements (`ChunkData::attributes`, `0x11`), following liblcf `ChunkAttribute`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// Element: `attribute_type` 0 = physical/weapon, 1 = magical. A–E ranks select
/// damage percentages, from most vulnerable to resistant; C defaults to 100%.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub id: u32,
    pub name: String,
    pub attribute_type: u32,
    pub a_rate: u32,
    pub b_rate: u32,
    pub c_rate: u32,
    pub d_rate: u32,
    pub e_rate: u32,
}

const ATTRIBUTE_SECTION: u32 = 0x11;
const ATTRIBUTE_NAME: u32 = 0x01;
const ATTRIBUTE_TYPE: u32 = 0x02;
const ATTRIBUTE_A_RATE: u32 = 0x0B;
const ATTRIBUTE_B_RATE: u32 = 0x0C;
const ATTRIBUTE_C_RATE: u32 = 0x0D;
const ATTRIBUTE_D_RATE: u32 = 0x0E;
const ATTRIBUTE_E_RATE: u32 = 0x0F;

// The editor omits default rates and never writes C, which remains neutral at 100%.
const ATTRIBUTE_DEFAULT_A_RATE: u32 = 300;
const ATTRIBUTE_DEFAULT_B_RATE: u32 = 200;
const ATTRIBUTE_DEFAULT_C_RATE: u32 = 100;
const ATTRIBUTE_DEFAULT_D_RATE: u32 = 50;
const ATTRIBUTE_DEFAULT_E_RATE: u32 = 0;

/// Parse the attribute table (`ChunkData::attributes` = `0x11`) out of an LDB
/// byte slice. Chunk ids (liblcf `ChunkAttribute`): name `0x01`, type `0x02`,
/// a_rate `0x0B`, b_rate `0x0C`, c_rate `0x0D`, d_rate `0x0E`, e_rate `0x0F`.
/// Each numeric field is a scalar integer chunk; omitted rates fall back to the
/// RM2000 default A–E grid.
pub fn parse_attributes(bytes: &[u8]) -> Result<Vec<Attribute>, LcfError> {
    let section = find_section(bytes, ATTRIBUTE_SECTION, LcfError::MissingAttributes)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut attributes = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut attribute = Attribute {
            id,
            name: String::new(),
            attribute_type: 0,
            a_rate: ATTRIBUTE_DEFAULT_A_RATE,
            b_rate: ATTRIBUTE_DEFAULT_B_RATE,
            c_rate: ATTRIBUTE_DEFAULT_C_RATE,
            d_rate: ATTRIBUTE_DEFAULT_D_RATE,
            e_rate: ATTRIBUTE_DEFAULT_E_RATE,
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                ATTRIBUTE_NAME => attribute.name = decode_cp1250(sub_data),
                ATTRIBUTE_TYPE => attribute.attribute_type = Reader::new(sub_data).varint()?,
                ATTRIBUTE_A_RATE => attribute.a_rate = Reader::new(sub_data).varint()?,
                ATTRIBUTE_B_RATE => attribute.b_rate = Reader::new(sub_data).varint()?,
                ATTRIBUTE_C_RATE => attribute.c_rate = Reader::new(sub_data).varint()?,
                ATTRIBUTE_D_RATE => attribute.d_rate = Reader::new(sub_data).varint()?,
                ATTRIBUTE_E_RATE => attribute.e_rate = Reader::new(sub_data).varint()?,
                _ => {}
            }
        }
        attributes.push(attribute);
    }
    Ok(attributes)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{element, make_ldb, section, subchunk, varint};
    use crate::{Attribute, LcfError, parse_attributes};

    #[test]
    fn parses_physical_attribute_with_full_rate_grid() {
        // Name bytes are CP1250 "Kard" (sword); a weapon attribute stores every
        // rate except C, which the editor never writes (defaults to 100).
        let sword = element(
            1,
            &[
                subchunk(0x01, b"Kard"),
                subchunk(0x02, &varint(0)),
                subchunk(0x0B, &varint(150)),
                subchunk(0x0C, &varint(125)),
                subchunk(0x0E, &varint(75)),
                subchunk(0x0F, &varint(50)),
            ],
        );
        let ldb = make_ldb(&[(0x0F, section(&[])), (0x11, section(&[sword]))]);
        let attributes = parse_attributes(&ldb).unwrap();
        assert_eq!(attributes.len(), 1);
        assert_eq!(
            attributes[0],
            Attribute {
                id: 1,
                name: "Kard".to_string(),
                attribute_type: 0,
                a_rate: 150,
                b_rate: 125,
                c_rate: 100,
                d_rate: 75,
                e_rate: 50,
            }
        );
    }

    #[test]
    fn magical_attribute_uses_default_grid_for_omitted_rates() {
        // Name bytes are CP1250 "Tűz" (fire): 0xFB = 'ű'. A magical element
        // overrides only A and B; C/D/E fall back to the default 100/50/0.
        let fire = element(
            1,
            &[
                subchunk(0x01, &[0x54, 0xFB, 0x7A]),
                subchunk(0x02, &varint(1)),
                subchunk(0x0B, &varint(200)),
                subchunk(0x0C, &varint(150)),
            ],
        );
        let ldb = make_ldb(&[(0x11, section(&[fire]))]);
        let attribute = &parse_attributes(&ldb).unwrap()[0];
        assert_eq!(attribute.name, "Tűz");
        assert_eq!(attribute.attribute_type, 1, "magical");
        assert_eq!(
            (
                attribute.a_rate,
                attribute.b_rate,
                attribute.c_rate,
                attribute.d_rate,
                attribute.e_rate,
            ),
            (200, 150, 100, 50, 0)
        );
    }

    #[test]
    fn attribute_defaults_to_full_grid_when_all_rates_omitted() {
        let bare = element(3, &[subchunk(0x01, b"Fold")]);
        let ldb = make_ldb(&[(0x11, section(&[bare]))]);
        let attribute = &parse_attributes(&ldb).unwrap()[0];
        assert_eq!(attribute.id, 3);
        assert_eq!(attribute.attribute_type, 0);
        assert_eq!(
            (
                attribute.a_rate,
                attribute.b_rate,
                attribute.c_rate,
                attribute.d_rate,
                attribute.e_rate,
            ),
            (300, 200, 100, 50, 0)
        );
    }

    #[test]
    fn parse_attributes_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(
            parse_attributes(&ldb),
            Err(LcfError::MissingAttributes)
        ));
    }
}
