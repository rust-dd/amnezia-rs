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

    #[test]
    fn original_airship_graphic_and_position_are_available() {
        let original = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../original");
        let database = std::fs::read(original.join("RPG_RT.ldb")).unwrap();
        let tree = std::fs::read(original.join("RPG_RT.lmt")).unwrap();
        let vehicles = parse_vehicles(&database, &tree).unwrap();
        assert!(!vehicles[2].charset.is_empty());
        assert!(vehicles[2].index < 8);
    }
}
