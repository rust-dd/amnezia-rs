//! Map-tree (`RPG_RT.lmt`) parsing: the party's starting position and the
//! map-info tree the game resolves each map's background music from. The tree is
//! a `[count]`-prefixed list of map-info entries (per entry a bare map id then a
//! chunk stream), then the tree order, the active node, and the party `Start`.

use crate::Music;
use crate::{LcfError, Reader, decode_cp1250};

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
        return Err(LcfError::BadSignature {
            expected: "LcfMapTree",
        });
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

    let mut start = Start {
        map_id: 0,
        x: 0,
        y: 0,
    };
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

/// One node of the LMT map-info tree, carried for BGM inheritance: the map `id`,
/// its `parent_map` (0 = the tree root), the `music_type` (0 = inherit the
/// parent, 1 = keep the current/event-set BGM, 2 = play `music`), and the map's
/// own `music` track. Chunk ids follow liblcf `ChunkMapInfo`: `parent_map` 0x02,
/// `music_type` 0x0B, `music` 0x0C.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapInfo {
    pub id: u32,
    pub parent_map: u32,
    pub music_type: u32,
    pub music: Music,
}

/// LMT's empty-name default keeps the current track, unlike LDB's silent `(OFF)`.
fn default_music() -> Music {
    Music {
        name: String::new(),
        volume: 100,
        tempo: 100,
        balance: 50,
        fadein: 0,
    }
}

/// Parse an LMT map-info `Music` sub-struct (chunk 0x0C): a bounded chunk stream
/// with the shared `ChunkMusic` ids — name 0x01, fadein 0x02, volume 0x03,
/// tempo 0x04, balance 0x05. Omitted fields keep [`default_music`]'s values.
fn parse_music(data: &[u8]) -> Result<Music, LcfError> {
    let mut reader = Reader::new(data);
    let mut music = default_music();
    while !reader.is_empty() {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let field = reader.take(size)?;
        match id {
            0x01 => music.name = decode_cp1250(field),
            0x02 => music.fadein = Reader::new(field).varint()?,
            0x03 => music.volume = Reader::new(field).varint()?,
            0x04 => music.tempo = Reader::new(field).varint()?,
            0x05 => music.balance = Reader::new(field).varint()?,
            _ => {}
        }
    }
    Ok(music)
}

/// Parse the map-info tree out of an LMT (`RPG_RT.lmt`): per map its `id`,
/// `parent_map` (0x02), `music_type` (0x0B), and `music` sub-struct (0x0C); the
/// other map-info chunks (name, layout, encounters, teleport/escape/save flags)
/// are skipped. The list is the same `[count]` header then per entry a bare map
/// `id` and a chunk stream that [`parse_start`] reads past, so this stops after
/// it — the tree order, active node, and party start that follow are not needed.
pub fn parse_map_infos(bytes: &[u8]) -> Result<Vec<MapInfo>, LcfError> {
    let mut reader = Reader::new(bytes);
    let signature_len = reader.byte()? as usize;
    let signature = reader.take(signature_len)?;
    if signature != b"LcfMapTree" {
        return Err(LcfError::BadSignature {
            expected: "LcfMapTree",
        });
    }
    let map_count = reader.varint()?;
    let mut infos = Vec::with_capacity(map_count as usize);
    for _ in 0..map_count {
        let id = reader.varint()?;
        let mut info = MapInfo {
            id,
            parent_map: 0,
            music_type: 0,
            music: default_music(),
        };
        loop {
            let chunk = reader.varint()?;
            if chunk == 0 {
                break;
            }
            let size = reader.varint()? as usize;
            let data = reader.take(size)?;
            match chunk {
                0x02 => info.parent_map = Reader::new(data).varint()?,
                0x0B => info.music_type = Reader::new(data).varint()?,
                0x0C => info.music = parse_music(data)?,
                _ => {}
            }
        }
        infos.push(info);
    }
    Ok(infos)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{subchunk, varint};
    use crate::{MapInfo, Music, Start, parse_map_infos, parse_start};

    #[test]
    fn parses_party_start() {
        // One map-info entry: count, map ID, empty chunk stream.
        let mut body = varint(1);
        body.extend(varint(1));
        body.extend(varint(0));
        // Tree order: count, node ID, active node.
        body.extend(varint(1));
        body.extend(varint(1));
        body.extend(varint(0));
        // Start: party map ID followed by the struct terminator.
        body.extend(subchunk(0x01, &varint(5)));
        body.extend(varint(0));
        let signature = b"LcfMapTree";
        let mut file = vec![signature.len() as u8];
        file.extend_from_slice(signature);
        file.extend_from_slice(&body);
        assert_eq!(
            parse_start(&file).unwrap(),
            Start {
                map_id: 5,
                x: 0,
                y: 0
            }
        );
    }

    #[test]
    fn parses_map_info_music_tree() {
        // Include unrelated name/save chunks to verify they do not disturb music parsing.
        let town_music = {
            let mut m = subchunk(0x01, b"Town");
            m.extend(subchunk(0x03, &varint(80)));
            m.push(0);
            m
        };
        let mut map1 = varint(1);
        map1.extend(subchunk(0x01, b"Town Map"));
        map1.extend(subchunk(0x02, &varint(0)));
        map1.extend(subchunk(0x0B, &varint(2)));
        map1.extend(subchunk(0x0C, &town_music));
        map1.extend(subchunk(0x21, &varint(1)));
        map1.push(0);
        let mut map2 = varint(2);
        map2.extend(subchunk(0x02, &varint(1)));
        map2.extend(subchunk(0x0B, &varint(0)));
        map2.push(0);

        let mut body = varint(2);
        body.extend_from_slice(&map1);
        body.extend_from_slice(&map2);
        // The tree order, active node, and party start follow in a real LMT;
        // parse_map_infos stops after the map-info list, so they can be omitted.
        let signature = b"LcfMapTree";
        let mut file = vec![signature.len() as u8];
        file.extend_from_slice(signature);
        file.extend_from_slice(&body);

        let infos = parse_map_infos(&file).unwrap();
        assert_eq!(infos.len(), 2);
        assert_eq!(
            infos[0],
            MapInfo {
                id: 1,
                parent_map: 0,
                music_type: 2,
                music: Music {
                    name: "Town".to_string(),
                    volume: 80,
                    tempo: 100,
                    balance: 50,
                    fadein: 0,
                },
            }
        );
        assert_eq!(
            (infos[1].id, infos[1].parent_map, infos[1].music_type),
            (2, 1, 0)
        );
        assert_eq!(infos[1].music.name, "");
    }
}
