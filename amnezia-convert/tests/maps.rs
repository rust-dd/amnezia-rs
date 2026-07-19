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

fn sub(id: u32, data: &[u8]) -> Vec<u8> {
    let mut out = varint(id);
    out.extend(varint(data.len() as u32));
    out.extend_from_slice(data);
    out
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

#[test]
fn move_fields_round_trip_through_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("move_round_trip");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    // One event whose only page carries explicit move chunks: type 2 (vertical),
    // frequency 5, speed 6. Convert to RON, read it back, and check they survive.
    let mut page = varint(1);
    page.extend(sub(0x1F, &varint(2)));
    page.extend(sub(0x20, &varint(5)));
    page.extend(sub(0x25, &varint(6)));
    page.push(0);
    let mut pages = varint(1);
    pages.extend_from_slice(&page);
    let mut event = varint(9);
    event.extend(sub(0x02, &varint(1)));
    event.extend(sub(0x03, &varint(1)));
    event.extend(sub(0x05, &pages));
    event.push(0);
    let mut section = varint(1);
    section.extend_from_slice(&event);
    let lmu = make_lmu(&[
        (0x02, varint(2)),
        (0x03, varint(1)),
        (0x47, layer_bytes(&[0, 0])),
        (0x48, layer_bytes(&[0, 0])),
        (0x51, section),
    ]);
    std::fs::write(input.join("Map0009.lmu"), lmu).unwrap();

    amnezia_convert::convert_maps(&input, &output).unwrap();
    let ron = std::fs::read_to_string(output.join("maps/map_0009.ron")).unwrap();
    let map: Map = ron::from_str(&ron).unwrap();
    let page = &map.events[0].pages[0];
    assert_eq!(
        (page.move_type, page.move_frequency, page.move_speed),
        (2, 5, 6)
    );
}

#[test]
fn map_ron_missing_move_fields_defaults_them() {
    // A map RON written before the move fields existed still loads: move_type
    // falls back to 0 (stationary) and frequency/speed to 3.
    let ron = r#"(chipset_id:1,width:1,height:1,lower:[0],upper:[10000],events:[
        (id:1,x:0,y:0,name:"",pages:[
            (trigger:0,graphic_name:"",graphic_index:0,layer:0,
             condition:(flags:0,switch_a:0,switch_b:0,variable_id:0,variable_value:0,item_id:0,actor_id:0),
             commands:[])
        ])
    ])"#;
    let map: Map = ron::from_str(ron).unwrap();
    let page = &map.events[0].pages[0];
    assert_eq!(
        (page.move_type, page.move_frequency, page.move_speed),
        (0, 3, 3)
    );
    // The pre-existing direction/pattern defaults still apply too.
    assert_eq!((page.direction, page.pattern), (2, 1));
}
