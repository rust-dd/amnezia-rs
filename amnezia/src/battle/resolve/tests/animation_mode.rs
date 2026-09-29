use super::*;

#[test]
fn original_skill_animations_follow_the_target_side_not_the_casters_side() {
    let mut battle = build_party2();
    let skills = crate::assets::load_ron::<Vec<SkillDef>>(&format!(
        "{}/skills.ron",
        crate::assets::asset_root()
    ));
    for (id, source, expected) in [
        (1, Source::Party(0), false),
        (2, Source::Party(0), false),
        (7, Source::Party(1), true),
        (9, Source::Party(1), true),
        (57, Source::Enemy(0), true),
        (58, Source::Enemy(0), true),
        (52, Source::Enemy(0), false),
        (53, Source::Enemy(0), false),
    ] {
        battle.pending_anims.clear();
        let skill = skills.iter().find(|s| s.id == id).unwrap();
        battle.push_skill_anim(source, skill, 0);
        assert_eq!(battle.pending_anims.len(), 1, "skill {id}");
        assert_eq!(battle.pending_anims[0].sound_only, expected, "skill {id}");
    }
}

#[test]
fn deferred_casts_and_confused_attacks_also_suppress_party_visuals() {
    for source in [Source::Party(0), Source::Enemy(0)] {
        let mut battle = build_party2();
        let mut skill = match source {
            Source::Party(_) => heal_skill(1, 20),
            Source::Enemy(_) => damage_skill(1, 20, vec![], vec![]),
        };
        skill.animation_id = 1;
        battle.skills = vec![skill];
        battle.start_test_action(Action {
            source,
            kind: Command::Skill {
                skill_id: 1,
                target: 0,
            },
            agility: 1,
        });
        assert!(battle.action_in_progress());
        assert!(battle.pending_anims[0].sound_only);
    }
    let mut battle = build_party2();
    battle.members[0].attack_animation = 1;
    battle.confused_attack(Source::Party(0), Source::Party(1));
    assert!(battle.pending_anims[0].sound_only);
    battle.pending_anims.clear();
    battle.push_anim(1, vec![battle.foe_anim_pos(0)]);
    assert!(
        !battle.pending_anims[0].sound_only,
        "event effects remain visible"
    );
}
