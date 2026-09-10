use super::*;
use amnezia_data::{AnimationDef, MonsterDef, SkillDef};

#[test]
fn original_battle_graphic_colors_and_default_animation_are_preserved() {
    let monsters = load_ron::<Vec<MonsterDef>>(&format!("{}/monsters.ron", asset_root()));
    assert_eq!(
        monsters
            .iter()
            .filter(|m| m.battler_hue != 0)
            .map(|m| (m.id, m.battler_hue))
            .collect::<Vec<_>>(),
        [
            (2, 120),
            (12, 180),
            (14, 60),
            (15, 30),
            (18, 180),
            (30, 330),
            (37, 330)
        ]
    );
    let animations = load_ron::<Vec<AnimationDef>>(&format!("{}/animations.ron", asset_root()));
    assert_eq!(
        animations
            .iter()
            .filter(|a| a
                .frames
                .iter()
                .flat_map(|f| &f.cells)
                .any(|c| c.valid && c.tone_gray != 100))
            .map(|a| a.id)
            .collect::<Vec<_>>(),
        [23, 46, 47, 55, 72]
    );
    let skills = load_ron::<Vec<SkillDef>>(&format!("{}/skills.ron", asset_root()));
    for id in [48, 65, 66] {
        assert_eq!(skills.iter().find(|s| s.id == id).unwrap().animation_id, 1);
    }
}
