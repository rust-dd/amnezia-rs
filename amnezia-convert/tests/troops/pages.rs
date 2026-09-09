use super::*;
use amnezia_data::TroopPageConditionDef;

#[test]
fn battle_event_pages_survive_the_full_conversion_pipeline() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_troop_pages_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    std::fs::create_dir_all(&input).unwrap();
    let mut condition = subchunk(0x01, &[0x7F]);
    for (field, value) in [
        (2, 545),
        (3, 617),
        (4, 8),
        (5, u32::MAX),
        (6, 2),
        (7, 1),
        (8, 10),
        (9, 90),
        (10, 3),
        (11, 1),
        (12, 0),
        (13, 4),
        (14, 5),
        (15, 40),
    ] {
        condition.extend(subchunk(field, &varint(value)));
    }
    condition.push(0);
    let mut command = [varint(13260), varint(1), varint(4)].concat();
    command.extend([0xC1, b't', b'o', b'k']);
    command.extend([varint(3), varint(4), varint(u32::MAX), varint(1)].concat());
    let mut pages = varint(2);
    pages.extend(element(
        1,
        &[subchunk(2, &condition), subchunk(0x0C, &command)],
    ));
    pages.extend(element(2, &[]));
    let bytes = make_ldb(0x0F, &[element(1, &[subchunk(0x0B, &pages)])]);
    std::fs::write(input.join("RPG_RT.ldb"), bytes).unwrap();
    assert_eq!(amnezia_convert::convert_troops(&input, &output).unwrap(), 1);
    let troops = ron::from_str::<Vec<TroopDef>>(
        &std::fs::read_to_string(output.join("troops.ron")).unwrap(),
    )
    .unwrap();
    assert_eq!(troops[0].pages.len(), 2);
    let page = &troops[0].pages[0];
    assert_eq!(
        page.condition,
        TroopPageConditionDef {
            flags: 0x7F,
            switch_a_id: 545,
            switch_b_id: 617,
            variable_id: 8,
            variable_value: -1,
            turn_a: 2,
            turn_b: 1,
            fatigue_min: 10,
            fatigue_max: 90,
            enemy_index: 3,
            enemy_hp_min: 1,
            enemy_hp_max: 0,
            actor_id: 4,
            actor_hp_min: 5,
            actor_hp_max: 40,
        }
    );
    assert_eq!((page.commands[0].code, page.commands[0].indent), (13260, 1));
    assert_eq!(page.commands[0].string, "Átok");
    assert_eq!(page.commands[0].params, [4, -1, 1]);
    assert_eq!(
        troops[0].pages[1].condition,
        TroopPageConditionDef::default()
    );
    assert!(troops[0].pages[1].commands.is_empty());
}

#[test]
fn old_troop_assets_without_pages_still_load() {
    let troop = ron::from_str::<TroopDef>("(id:1,name:\"Old\",members:[])").unwrap();
    assert!(troop.pages.is_empty());
    assert_eq!(
        ron::from_str::<TroopPageConditionDef>("()").unwrap(),
        TroopPageConditionDef::default()
    );
}
