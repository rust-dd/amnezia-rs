use super::*;

fn original_skills(battle: &mut Battle) {
    battle.skills = crate::assets::load_ron(&format!("{}/skills.ron", crate::assets::asset_root()));
}

#[test]
fn original_right_cannon_and_reaper_rockets_hit_each_living_hero() {
    for id in [57, 58] {
        let mut battle = build_party2();
        original_skills(&mut battle);
        for member in &mut battle.members {
            member.hp = 1000;
            member.max_hp = 1000;
        }
        battle.members[1].defending = true;
        battle.enemy_cast(0, id, 0).unwrap();
        assert!(battle.members[0].hp < 1000);
        assert_eq!(battle.members[1].hp, 1000);
        assert_eq!(battle.pending_anims.len(), 1);
        assert_eq!(battle.pending_anims[0].targets.len(), 2);
        while battle.resolve_next() {}
        assert!(battle.members[1].hp < 1000);
        assert!(1000 - battle.members[1].hp < 1000 - battle.members[0].hp);
        assert_eq!(battle.pending_numbers.len(), 2);
        assert_eq!(battle.pending_anims.len(), 1);
    }
}

#[test]
fn a_dead_multi_target_recipient_is_skipped_without_hitting_someone_twice() {
    let mut battle = build_party2();
    let mut skill = damage_skill(1, 1, vec![], vec![]);
    skill.scope = 1;
    skill.magical_rate = 0;
    skill.variance = 0;
    battle.skills = vec![skill];
    battle.enemy_cast(0, 1, 0).unwrap();
    let first_hp = battle.members[0].hp;
    battle.members[1].hp = 0;
    while battle.resolve_next() {}
    assert_eq!(battle.members[0].hp, first_hp);
    assert_eq!(battle.members[1].hp, 0);
    assert_eq!(battle.pending_numbers.len(), 1);
}

#[test]
fn an_animated_multi_target_enemy_cast_pays_once_and_switches_after_the_last_target() {
    let mut battle = build_party2();
    let mut skill = damage_skill(1, 1, vec![], vec![]);
    skill.scope = 1;
    skill.animation_id = 3;
    battle.skills = vec![skill];
    battle.enemies[0].sp = 3;
    battle.enemies[0].switch_on_after_action = Some(7);
    battle.queue = vec![Action {
        source: Source::Enemy(0),
        kind: Command::Skill {
            skill_id: 1,
            target: 0,
        },
        agility: 1,
    }];
    battle.resolve_next();
    assert_eq!(battle.enemies[0].sp, 0);
    assert!(battle.pending_switches.is_empty());
    battle.resolve_next();
    assert!(battle.pending_switches.is_empty());
    battle.resolve_next();
    assert_eq!(battle.pending_switches, [(7, true)]);
    assert_eq!(battle.enemies[0].sp, 0);
    assert_eq!(battle.pending_anims.len(), 1);
}

#[test]
fn original_probe_heals_its_whole_living_team() {
    let mut battle = build_1v2();
    original_skills(&mut battle);
    battle.enemies[0].hp = 5;
    battle.enemies[1].hp = 10;
    battle.enemy_cast(0, 52, 0).unwrap();
    assert_eq!(battle.enemies[0].hp, 30);
    assert_eq!(battle.enemies[1].hp, 10);
    assert_eq!(battle.pending_anims[0].targets.len(), 2);
    while battle.resolve_next() {}
    assert_eq!(battle.enemies[1].hp, 30);
    assert_eq!(battle.pending_numbers.len(), 2);
}

#[test]
fn original_left_cannon_repairs_and_revives_its_chosen_ally_not_itself() {
    for hp in [0, 10, 2000] {
        let mut battle = build_1v2();
        original_skills(&mut battle);
        battle.enemies[0].hp = 5;
        battle.enemies[1].hp = hp;
        battle.enemies[1].max_hp = 2000;
        if hp == 0 {
            battle.enemies[1].states = vec![(1, 0)];
            battle.start_foe_death(1, false);
        }
        battle.enemy_cast(0, 53, 1).unwrap();
        assert_eq!(battle.enemies[0].hp, 5);
        assert_eq!(battle.enemies[1].hp, (hp + 999).min(2000));
        assert!(battle.enemies[1].dying.is_none());
        assert!(!logic::has_state(&battle.enemies[1].states, 1));
        assert_eq!(battle.pending_anims[0].targets, [battle.foe_anim_pos(1)]);
    }
}

#[test]
fn self_scope_ignores_the_stored_target_and_reviving_without_hp_uses_a_percentage() {
    let mut battle = build_1v2();
    let mut skill = heal_skill(1, 25);
    skill.magical_rate = 0;
    skill.variance = 0;
    skill.scope = 2;
    battle.skills = vec![skill.clone()];
    battle.enemies[0].hp = 1;
    battle.enemies[1].hp = 1;
    battle.enemy_cast(0, 1, 1).unwrap();
    assert_eq!((battle.enemies[0].hp, battle.enemies[1].hp), (26, 1));
    skill.scope = 3;
    skill.affect_hp = false;
    skill.affected_states = vec![1];
    battle.skills = vec![skill];
    battle.enemies[1].hp = 0;
    battle.enemies[1].max_hp = 200;
    battle.enemy_cast(0, 1, 1).unwrap();
    assert_eq!((battle.enemies[0].hp, battle.enemies[1].hp), (26, 50));
}

#[test]
fn support_does_not_heal_fled_enemies_or_revive_without_the_death_flag() {
    for fled in [false, true] {
        let mut battle = build_1v2();
        original_skills(&mut battle);
        battle.enemies[1].hp = if fled { 5 } else { 0 };
        battle.enemies[1].fled = fled;
        battle.enemy_cast(0, 52, 0).unwrap();
        while battle.resolve_next() {}
        assert_eq!(battle.enemies[1].hp, if fled { 5 } else { 0 });
        assert_eq!(battle.pending_anims[0].targets.len(), 1);
    }
}
