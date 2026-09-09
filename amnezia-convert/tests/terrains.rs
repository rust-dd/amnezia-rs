use amnezia_data::TerrainDef;
use std::path::Path;

#[test]
fn terrain_fields_round_trip_through_the_converter() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("terrain_round_trip");
    let input = directory.join("in");
    let output = directory.join("out");
    std::fs::create_dir_all(&input).unwrap();
    let mut section = vec![2, 1, 0, 2];
    for (field, value) in [(2, 2), (3, 0), (5, 1), (6, 1), (7, 0), (9, 0), (11, 3)] {
        section.extend([field, 1, value]);
    }
    section.extend([1, 1, 0xF5, 4, 1, b'F', 0]);
    let mut ldb = vec![11];
    ldb.extend(b"LcfDataBase");
    ldb.extend([0x10, section.len() as u8]);
    ldb.extend(section);
    ldb.push(0);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();
    assert_eq!(
        amnezia_convert::convert_terrains(&input, &output).unwrap(),
        2
    );
    let text = std::fs::read_to_string(output.join("terrains.ron")).unwrap();
    let terrains = ron::from_str::<Vec<TerrainDef>>(&text).unwrap();
    assert_eq!(
        terrains[0],
        TerrainDef {
            id: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        terrains[1],
        TerrainDef {
            id: 2,
            name: "ő".into(),
            damage: 2,
            encounter_rate: 0,
            background_name: "F".into(),
            boat_pass: true,
            ship_pass: true,
            airship_pass: false,
            airship_land: false,
            bush_depth: 3,
        }
    );
    let defaults = ron::from_str::<TerrainDef>("(id:1)").unwrap();
    assert_eq!(defaults, terrains[0]);
}
