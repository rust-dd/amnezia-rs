use amnezia_data::{AnimationCellDef, AnimationDef, AnimationFrameDef, AnimationTimingDef};
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

// A count-prefixed struct-list, the shape RM2000 uses for the nested frame,
// cell, and timing lists inside an animation.
fn section(elements: &[Vec<u8>]) -> Vec<u8> {
    let mut out = varint(elements.len() as u32);
    for e in elements {
        out.extend_from_slice(e);
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
fn converts_ldb_to_animations_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_animations_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    // One cell: tile 3, offset x=-24 (a signed field stored as a wrapped
    // varint) y=48, zoomed to 200%, 40% transparent, neutral tone.
    let cell = element(
        1,
        &[
            subchunk(0x02, &varint(3)),
            subchunk(0x03, &varint((-24i32) as u32)),
            subchunk(0x04, &varint(48)),
            subchunk(0x05, &varint(200)),
            subchunk(0x0A, &varint(40)),
        ],
    );
    let frames = section(&[element(1, &[subchunk(0x01, &section(&[cell]))])]);

    // A timing at frame 5: plays "Punch" (name-only Sound struct) and flashes
    // the whole screen (scope 2) with red 28, the other channels defaulting.
    let mut sound = subchunk(0x01, b"Punch");
    sound.extend(varint(0));
    let timing = element(
        1,
        &[
            subchunk(0x01, &varint(5)),
            subchunk(0x02, &sound),
            subchunk(0x03, &varint(2)),
            subchunk(0x04, &varint(28)),
        ],
    );
    let timings = section(&[timing]);

    // Name bytes are CP1250 "Tűz" (fire): 0xFB = 'ű'.
    let anim = element(
        1,
        &[
            subchunk(0x01, &[0x54, 0xFB, 0x7A]),
            subchunk(0x02, b"Fire1"),
            subchunk(0x06, &timings),
            subchunk(0x09, &varint(1)),
            subchunk(0x0A, &varint(2)),
            subchunk(0x0C, &frames),
        ],
    );
    let ldb = make_ldb(0x13, &[anim]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_animations(&input, &output).unwrap();
    assert_eq!(count, 1);

    let text = std::fs::read_to_string(output.join("animations.ron")).unwrap();
    let animations: Vec<AnimationDef> = ron::from_str(&text).unwrap();
    assert_eq!(
        animations[0],
        AnimationDef {
            id: 1,
            name: "Tűz".to_string(),
            animation_name: "Fire1".to_string(),
            scope: 1,
            position: 2,
            frames: vec![AnimationFrameDef {
                cells: vec![AnimationCellDef {
                    valid: true,
                    cell_id: 3,
                    x: -24,
                    y: 48,
                    scale: 200,
                    tone_red: 100,
                    tone_green: 100,
                    tone_blue: 100,
                    tone_gray: 100,
                    transparency: 40,
                }],
            }],
            timings: vec![AnimationTimingDef {
                frame: 5,
                se_name: "Punch".to_string(),
                se_volume: 100,
                se_tempo: 100,
                flash_scope: 2,
                flash_red: 28,
                flash_green: 31,
                flash_blue: 31,
                flash_power: 31,
            }],
        }
    );
}
