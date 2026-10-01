use super::*;
use amnezia_data::ActorDef;

#[test]
fn new_game_ron_knows_his_original_x_strike() {
    let actors = load_ron::<Vec<ActorDef>>(&format!("{}/actors.ron", asset_root()));
    let ron = actors.iter().find(|actor| actor.id == 1).unwrap();
    let progression = crate::progression::Progression::default();
    assert_eq!(progression.known_skill_ids(ron), [1]);
}

#[test]
fn original_actor_graphics_commands_and_unarmed_animations_are_preserved() {
    let actors = load_ron::<Vec<ActorDef>>(&format!("{}/actors.ron", asset_root()));
    let expected = [
        (1, "Chara1", 0, "Pengetánc", 1),
        (2, "Chara1", 1, "Varázsdal", 9),
        (3, "Chara4", 2, "Tigrisharc", 19),
        (4, "Chara4", 0, "Kombó", 25),
        (5, "Chara1", 2, "", 32),
        (6, "Chara3", 2, "Shin-Ra-Ta", 35),
        (7, "Chara4", 4, "Gyógyítás", 42),
        (8, "Chara2", 2, "Draco", 48),
        (9, "Monster2", 3, "Pusztítás", 55),
        (10, "Chara4", 2, "Tigrisharc", 19),
    ];
    assert_eq!(actors.len(), expected.len());
    for (id, charset, index, skill, animation) in expected {
        let actor = actors.iter().find(|a| a.id == id).unwrap();
        assert_eq!(actor.character_name, charset, "actor {id}");
        assert_eq!(actor.character_index, index, "actor {id}");
        assert!(actor.rename_skill, "actor {id}");
        assert_eq!(actor.skill_name, skill, "actor {id}");
        assert_eq!(actor.unarmed_animation, animation, "actor {id}");
        graphic_exists("CharSet", charset, &format!("actor {id}"));
    }
}
