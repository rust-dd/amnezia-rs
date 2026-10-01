use crate::{LcfError, Reader, decode_cp1250};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vehicle {
    pub charset: String,
    pub index: u32,
    pub map_id: u32,
    pub x: u32,
    pub y: u32,
}

/// Boat, ship, and airship graphics from the database and positions from the map tree.
pub fn parse_vehicles(database: &[u8], tree: &[u8]) -> Result<[Vehicle; 3], LcfError> {
    let mut vehicles = std::array::from_fn(|_| Vehicle::default());
    let system = crate::database::find_section(database, 0x16, LcfError::MissingSystem)?;
    read_chunks(&mut Reader::new(system), |id, field| {
        match id {
            0x0B..=0x0D => vehicles[(id - 0x0B) as usize].charset = decode_cp1250(field),
            0x0E..=0x10 => vehicles[(id - 0x0E) as usize].index = Reader::new(field).varint()?,
            _ => {}
        }
        Ok(())
    })?;

    let mut reader = Reader::new(tree);
    let len = reader.byte()? as usize;
    if reader.take(len)? != b"LcfMapTree" {
        return Err(LcfError::BadSignature {
            expected: "LcfMapTree",
        });
    }
    for _ in 0..reader.varint()? {
        reader.varint()?;
        read_chunks(&mut reader, |_, _| Ok(()))?;
    }
    for _ in 0..reader.varint()? {
        reader.varint()?;
    }
    reader.varint()?;
    read_chunks(&mut reader, |id, field| {
        for (vehicle, base) in vehicles.iter_mut().zip([0x0B, 0x15, 0x1F]) {
            if (base..=base + 2).contains(&id) {
                let value = Reader::new(field).varint()?;
                match id - base {
                    0 => vehicle.map_id = value,
                    1 => vehicle.x = value,
                    _ => vehicle.y = value,
                }
            }
        }
        Ok(())
    })?;
    Ok(vehicles)
}

fn read_chunks(
    reader: &mut Reader,
    mut visit: impl FnMut(u32, &[u8]) -> Result<(), LcfError>,
) -> Result<(), LcfError> {
    while !reader.is_empty() {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        visit(id, reader.take(size)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{element, make_ldb, section, subchunk, varint};

    fn map_tree(start: &[u8]) -> Vec<u8> {
        let signature = b"LcfMapTree";
        let mut tree = vec![signature.len() as u8];
        tree.extend_from_slice(signature);
        tree.extend(section(&[
            element(129, &[subchunk(0x01, b"Harbor"), subchunk(0x0B, &[2])]),
            element(511, &[subchunk(0x02, &varint(129))]),
        ]));
        for value in [2, 511, 129, 511] {
            tree.extend(varint(value));
        }
        for (id, value) in [(0x01, 511), (0x02, 8), (0x03, 9)] {
            tree.extend(subchunk(id, &varint(value)));
        }
        tree.extend_from_slice(start);
        tree.push(0);
        tree
    }

    #[test]
    fn parses_all_vehicle_graphics_and_start_positions() {
        let system = [
            subchunk(0x0B, b"Cs\xf3nak"),
            subchunk(0x0C, b"Ship"),
            subchunk(0x0D, b"Airship"),
            subchunk(0x0E, &varint(1)),
            subchunk(0x0F, &varint(4)),
            subchunk(0x10, &varint(7)),
            subchunk(0x48, &varint(1)),
            vec![0],
        ]
        .concat();
        let database = make_ldb(&[(0x02, vec![0]), (0x16, system)]);
        let start = [
            (0x0B, 129),
            (0x0C, 132),
            (0x0D, 7),
            (0x15, 250),
            (0x16, 8),
            (0x17, 144),
            (0x1F, 511),
            (0x20, 66),
            (0x21, 33),
            (0x30, 17),
        ]
        .into_iter()
        .flat_map(|(id, value)| subchunk(id, &varint(value)))
        .collect::<Vec<_>>();

        assert_eq!(
            parse_vehicles(&database, &map_tree(&start)).unwrap(),
            [
                Vehicle {
                    charset: "Csónak".into(),
                    index: 1,
                    map_id: 129,
                    x: 132,
                    y: 7,
                },
                Vehicle {
                    charset: "Ship".into(),
                    index: 4,
                    map_id: 250,
                    x: 8,
                    y: 144,
                },
                Vehicle {
                    charset: "Airship".into(),
                    index: 7,
                    map_id: 511,
                    x: 66,
                    y: 33,
                },
            ]
        );
    }

    #[test]
    fn omitted_vehicle_fields_keep_defaults() {
        let database = make_ldb(&[(0x16, vec![0])]);
        assert_eq!(
            parse_vehicles(&database, &map_tree(&[])).unwrap(),
            std::array::from_fn(|_| Vehicle::default())
        );
    }

    #[test]
    fn invalid_headers_and_missing_system_are_rejected() {
        let database = make_ldb(&[(0x16, vec![0])]);
        assert!(matches!(
            parse_vehicles(&[0], &map_tree(&[])),
            Err(LcfError::BadSignature {
                expected: "LcfDataBase"
            })
        ));
        assert!(matches!(
            parse_vehicles(&database, &[0]),
            Err(LcfError::BadSignature {
                expected: "LcfMapTree"
            })
        ));
        assert!(matches!(
            parse_vehicles(&make_ldb(&[]), &map_tree(&[])),
            Err(LcfError::MissingSystem)
        ));
    }

    #[test]
    fn truncated_vehicle_chunks_and_values_are_rejected() {
        for system in [subchunk(0x0E, &[0x80]), vec![0x0D, 2, b'A']] {
            assert!(matches!(
                parse_vehicles(&make_ldb(&[(0x16, system)]), &map_tree(&[])),
                Err(LcfError::UnexpectedEof)
            ));
        }
        let database = make_ldb(&[(0x16, vec![0])]);
        assert!(matches!(
            parse_vehicles(&database, &map_tree(&subchunk(0x1F, &[0x80]))),
            Err(LcfError::UnexpectedEof)
        ));
    }

    #[test]
    #[ignore = "requires the untracked original/RPG_RT.ldb and original/RPG_RT.lmt"]
    fn original_airship_graphic_and_position_are_available() {
        let original = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../original");
        let database = std::fs::read(original.join("RPG_RT.ldb")).unwrap();
        let tree = std::fs::read(original.join("RPG_RT.lmt")).unwrap();
        let vehicles = parse_vehicles(&database, &tree).unwrap();
        assert!(!vehicles[2].charset.is_empty());
        assert!(vehicles[2].index < 8);
    }
}
