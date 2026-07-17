use amnezia_data::Map;
use std::path::Path;

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

fn make_lmu(chunks: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let sig = b"LcfMapUnit";
    let mut out = vec![sig.len() as u8];
    out.extend_from_slice(sig);
    for (id, data) in chunks {
        out.extend(varint(*id));
        out.extend(varint(data.len() as u32));
        out.extend_from_slice(data);
    }
    out.push(0);
    out
}

fn layer_bytes(tiles: &[u16]) -> Vec<u8> {
    tiles.iter().flat_map(|t| t.to_le_bytes()).collect()
}

#[test]
fn converts_lmu_to_map_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_lmu");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    let lmu = make_lmu(&[
        (0x01, varint(7)),
        (0x02, varint(2)),
        (0x03, varint(1)),
        (0x47, layer_bytes(&[1, 2])),
        (0x48, layer_bytes(&[10, 11])),
    ]);
    std::fs::write(input.join("Map0007.lmu"), lmu).unwrap();

    let count = amnezia_convert::convert_maps(&input, &output).unwrap();
    assert_eq!(count, 1);

    let ron = std::fs::read_to_string(output.join("maps/map_0007.ron")).unwrap();
    let map: Map = ron::from_str(&ron).unwrap();
    assert_eq!(
        map,
        Map {
            chipset_id: 7,
            width: 2,
            height: 1,
            lower: vec![1, 2],
            upper: vec![10, 11],
            events: vec![]
        }
    );
}
