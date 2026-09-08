use amnezia_data::{MusicDef, SoundDef, SystemDef};
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

/// A nested `Music`/`Sound` struct: its sub-chunks then the struct terminator.
fn nested(subchunks: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for c in subchunks {
        out.extend_from_slice(c);
    }
    out.extend(varint(0));
    out
}

/// Build an LDB whose only section is the single-struct system section (0x16):
/// a bare chunk stream, no `[count]` header.
fn make_ldb(system: &[u8]) -> Vec<u8> {
    let sig = b"LcfDataBase";
    let mut out = vec![sig.len() as u8];
    out.extend_from_slice(sig);
    out.extend(varint(0x16));
    out.extend(varint(system.len() as u32));
    out.extend_from_slice(system);
    out.push(0);
    out
}

#[test]
fn converts_ldb_to_system_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_system_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    // battle_music (0x20): "Battle2" volume 90 tempo 100; battle_end_music (0x21):
    // "Victory"; gameover_music (0x26): "GameOver"; a few battle SE.
    let mut section = Vec::new();
    section.extend(subchunk(0x48, &varint(1)));
    section.extend(subchunk(
        0x20,
        &nested(&[subchunk(0x01, b"Battle2"), subchunk(0x03, &varint(90))]),
    ));
    section.extend(subchunk(0x21, &nested(&[subchunk(0x01, b"Victory")])));
    section.extend(subchunk(0x26, &nested(&[subchunk(0x01, b"GameOver")])));
    section.extend(subchunk(0x2D, &nested(&[subchunk(0x01, b"Battle_Start")])));
    section.extend(subchunk(0x2E, &nested(&[subchunk(0x01, b"Escape")])));
    section.extend(subchunk(0x30, &nested(&[subchunk(0x01, b"Damage1")])));
    section.extend(subchunk(0x31, &nested(&[subchunk(0x01, b"Damage2")])));
    section.extend(subchunk(0x32, &nested(&[subchunk(0x01, b"Miss")])));
    section.extend(subchunk(0x33, &nested(&[subchunk(0x01, b"Monster1")])));

    std::fs::write(input.join("RPG_RT.ldb"), make_ldb(&section)).unwrap();

    amnezia_convert::convert_system(&input, &output).unwrap();

    let text = std::fs::read_to_string(output.join("system.ron")).unwrap();
    let system = ron::from_str::<SystemDef>(&text).unwrap();
    assert_eq!(system.font_id, 1);

    assert_eq!(
        system.battle_music,
        MusicDef {
            name: "Battle2".to_string(),
            volume: 90,
            tempo: 100,
            balance: 50,
            fadein: 0,
        }
    );
    assert_eq!(system.battle_end_music.name, "Victory");
    assert_eq!(system.gameover_music.name, "GameOver");
    assert_eq!(system.battle_se.name, "Battle_Start");
    assert_eq!(system.escape_se.name, "Escape");
    assert_eq!(system.enemy_damaged_se.name, "Damage1");
    assert_eq!(system.actor_damaged_se.name, "Damage2");
    assert_eq!(system.dodge_se.name, "Miss");
    // enemy_death_se (0x33) round-trips into enemy_defeated_se.
    assert_eq!(
        system.enemy_defeated_se,
        SoundDef {
            name: "Monster1".to_string(),
            volume: 100,
            tempo: 100,
            balance: 50,
        }
    );
    // An absent field keeps the RM2000 silent default.
    assert_eq!(system.title_music.name, "(OFF)");
}
