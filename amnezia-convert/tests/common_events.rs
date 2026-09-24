use amnezia_data::{CommonEvent, EventCommand};
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

fn subchunk(id: u32, data: &[u8]) -> Vec<u8> {
    let mut out = varint(id);
    out.extend(varint(data.len() as u32));
    out.extend_from_slice(data);
    out
}

fn element(id: u32, subchunks: &[Vec<u8>]) -> Vec<u8> {
    let mut out = varint(id);
    for chunk in subchunks {
        out.extend_from_slice(chunk);
    }
    out.extend(varint(0));
    out
}

/// One record of the flat command stream: `[code][indent][strlen][string]
/// [paramcount][params]`.
fn command(code: u32, string: &[u8], params: &[u32]) -> Vec<u8> {
    let mut out = varint(code);
    out.extend(varint(0));
    out.extend(varint(string.len() as u32));
    out.extend_from_slice(string);
    out.extend(varint(params.len() as u32));
    for &p in params {
        out.extend(varint(p));
    }
    out
}

fn make_ldb(section_id: u32, elements: &[Vec<u8>]) -> Vec<u8> {
    let mut section = varint(elements.len() as u32);
    for e in elements {
        section.extend_from_slice(e);
    }
    let sig = b"LcfDataBase";
    let mut out = vec![sig.len() as u8];
    out.extend_from_slice(sig);
    out.extend(varint(section_id));
    out.extend(varint(section.len() as u32));
    out.extend_from_slice(&section);
    out.push(0);
    out
}

#[test]
fn converts_ldb_to_common_events_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_common_events_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    let mut commands = command(10110, &[0x48, 0x65, 0x6C, 0x6C, 0xF3], &[]);
    commands.extend(command(0, b"", &[]));
    let intro = element(
        1,
        &[
            subchunk(0x01, &[0x4B, 0x65, 0x7A, 0x64, 0xE9, 0x73]),
            subchunk(0x0B, &varint(3)),
            subchunk(0x0C, &varint(1)),
            subchunk(0x0D, &varint(7)),
            subchunk(0x16, &commands),
        ],
    );
    let idle = element(2, &[subchunk(0x01, b"Idle")]);
    let ldb = make_ldb(0x19, &[intro, idle]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_common_events(&input, &output).unwrap();
    assert_eq!(count, 2);

    let text = std::fs::read_to_string(output.join("common_events.ron")).unwrap();
    let events = ron::from_str::<Vec<CommonEvent>>(&text).unwrap();
    assert_eq!(
        events[0],
        CommonEvent {
            id: 1,
            name: "Kezdés".to_string(),
            trigger: 3,
            switch_flag: true,
            switch_id: 7,
            commands: vec![
                EventCommand {
                    code: 10110,
                    indent: 0,
                    string: "Helló".to_string(),
                    params: vec![]
                },
                EventCommand {
                    code: 0,
                    indent: 0,
                    string: String::new(),
                    params: vec![]
                },
            ],
        }
    );
    assert_eq!(events[1].name, "Idle");
    assert_eq!(events[1].trigger, 5, "trigger defaults to call");
    assert!(!events[1].switch_flag);
    assert_eq!(events[1].switch_id, 1);
    assert!(events[1].commands.is_empty());
}
