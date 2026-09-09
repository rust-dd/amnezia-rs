use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terrain {
    pub id: u32,
    pub name: String,
    pub damage: i32,
    pub encounter_rate: u32,
    pub background_name: String,
    pub boat_pass: bool,
    pub ship_pass: bool,
    pub airship_pass: bool,
    pub airship_land: bool,
    pub bush_depth: u32,
}

pub fn parse_terrains(bytes: &[u8]) -> Result<Vec<Terrain>, LcfError> {
    let section = find_section(bytes, 0x10, LcfError::MissingTerrains)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut terrains = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mut terrain = Terrain {
            id: reader.varint()?,
            name: String::new(),
            damage: 0,
            encounter_rate: 100,
            background_name: String::new(),
            boat_pass: false,
            ship_pass: false,
            airship_pass: true,
            airship_land: true,
            bush_depth: 0,
        };
        loop {
            let id = reader.varint()?;
            if id == 0 {
                break;
            }
            let size = reader.varint()? as usize;
            let data = reader.take(size)?;
            match id {
                1 => terrain.name = decode_cp1250(data),
                2 => terrain.damage = Reader::new(data).varint()? as i32,
                3 => terrain.encounter_rate = Reader::new(data).varint()?,
                4 => terrain.background_name = decode_cp1250(data),
                5 => terrain.boat_pass = Reader::new(data).varint()? != 0,
                6 => terrain.ship_pass = Reader::new(data).varint()? != 0,
                7 => terrain.airship_pass = Reader::new(data).varint()? != 0,
                9 => terrain.airship_land = Reader::new(data).varint()? != 0,
                11 => terrain.bush_depth = Reader::new(data).varint()?,
                _ => {}
            }
        }
        terrains.push(terrain);
    }
    Ok(terrains)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{element, make_ldb, section, subchunk};

    #[test]
    fn terrain_defaults_and_explicit_zero_flags_are_distinct() {
        let ldb = make_ldb(&[(
            0x10,
            section(&[
                element(1, &[]),
                element(
                    2,
                    &[
                        subchunk(1, &[0xF5]),
                        subchunk(2, &[2]),
                        subchunk(3, &[0]),
                        subchunk(4, b"Forest"),
                        subchunk(5, &[1]),
                        subchunk(6, &[1]),
                        subchunk(7, &[0]),
                        subchunk(9, &[0]),
                        subchunk(11, &[3]),
                    ],
                ),
            ]),
        )]);
        let terrains = parse_terrains(&ldb).unwrap();
        assert_eq!(
            terrains[0],
            Terrain {
                id: 1,
                name: String::new(),
                damage: 0,
                encounter_rate: 100,
                background_name: String::new(),
                boat_pass: false,
                ship_pass: false,
                airship_pass: true,
                airship_land: true,
                bush_depth: 0,
            }
        );
        assert_eq!(
            terrains[1],
            Terrain {
                id: 2,
                name: "ő".into(),
                damage: 2,
                encounter_rate: 0,
                background_name: "Forest".into(),
                boat_pass: true,
                ship_pass: true,
                airship_pass: false,
                airship_land: false,
                bush_depth: 3,
            }
        );
    }

    #[test]
    fn missing_terrain_section_and_truncated_values_are_errors() {
        assert!(matches!(
            parse_terrains(&make_ldb(&[])),
            Err(LcfError::MissingTerrains)
        ));
        let ldb = make_ldb(&[(0x10, section(&[element(1, &[subchunk(11, &[0x80])])]))]);
        assert!(matches!(parse_terrains(&ldb), Err(LcfError::UnexpectedEof)));
    }
}
