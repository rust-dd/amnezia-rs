use super::*;
use amnezia_data::{SkillDef, StateDef};

#[test]
fn original_state_colors_and_actor_enemy_messages_are_preserved() {
    let states = load_ron::<Vec<StateDef>>(&format!("{}/states.ron", asset_root()));
    assert_eq!(
        states.iter().map(|s| s.color).collect::<Vec<_>>(),
        [3, 6, 10, 18, 4, 7, 8, 8, 9, 9]
    );
    assert!(
        states
            .iter()
            .all(|s| s.message_already.is_empty() && s.message_affected.is_empty())
    );
    assert_eq!(states[0].message_actor, " összeesik!");
    assert_eq!(states[0].message_recovery, " visszanyeri eszméletét!");
    assert_eq!(states[1].message_actor, " mérgezést kap");
    assert_eq!(states[1].message_enemy, " megmérgeződik");
    assert_eq!(states[6].message_actor, " mély álomba zuhan");
    assert_eq!(states[6].message_enemy, " elalszik");
    assert_eq!(states[6].message_recovery, " felébred");
}

#[test]
fn original_skill_use_lines_keep_their_leading_spaces_and_blank_separators() {
    let skills = load_ron::<Vec<SkillDef>>(&format!("{}/skills.ron", asset_root()));
    assert_eq!(
        skills
            .iter()
            .filter(|s| !s.using_message1.is_empty())
            .count(),
        68
    );
    assert!(skills.iter().all(|s| s.using_message2.is_empty()));
    for (id, expected) in [
        (1, " X-csapást alkalmaz"),
        (2, " ciklonként támad"),
        (7, " elénekli a Gyógyító dallamot"),
        (48, ""),
        (52, " javít mindenen"),
        (65, " SP-t szív el"),
        (66, ""),
    ] {
        assert_eq!(
            skills.iter().find(|s| s.id == id).unwrap().using_message1,
            expected
        );
    }
}
