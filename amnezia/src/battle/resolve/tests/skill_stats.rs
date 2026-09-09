use super::*;

fn original_skill(id: u32) -> SkillDef {
    crate::assets::load_ron::<Vec<SkillDef>>(&format!("{}/skills.ron", crate::assets::asset_root()))
        .into_iter()
        .find(|skill| skill.id == id)
        .unwrap()
}

fn stats(value: u32) -> logic::Stats {
    logic::Stats {
        attack: value,
        defense: value,
        spirit: value,
        agility: value,
    }
}

#[test]
fn original_phoenix_drains_hp_sp_and_all_four_stats() {
    let mut battle = build_1v2();
    let skill = original_skill(6);
    assert!(skill.ignore_defense);
    assert_eq!(skill.affect_stats, [true; 4]);
    battle.enemies[0].stats = stats(200);
    battle.enemies[0].hp = 2000;
    battle.enemies[0].max_hp = 2000;
    battle.enemies[0].sp = 2000;
    battle.skill_hit_enemy(0, 0, &skill);
    assert_eq!((battle.enemies[0].hp, battle.enemies[0].sp), (1001, 1001));
    assert_eq!(battle.enemies[0].stat_modifiers, [-100; 4]);
    assert_eq!(battle.battler_stats(Source::Enemy(0)), stats(100));
}

#[test]
fn original_tiger_roar_changes_only_attack_and_defense_with_separate_success_rolls() {
    let mut battle = build_1v2();
    let skill = original_skill(16);
    assert_eq!(skill.affect_stats, [true, true, false, false]);
    battle.members[0].stats.spirit = 80;
    battle.enemies[0].stats = stats(160);
    let hp = battle.enemies[0].hp;
    let mut both = false;
    let mut partial = false;
    for seed in 0..100 {
        battle.rng = seed;
        battle.enemies[0].stat_modifiers = [0; 4];
        battle.skill_hit_enemy(0, 0, &skill);
        let [attack, defense, spirit, agility] = battle.enemies[0].stat_modifiers;
        assert!(attack == 0 || (-12..=-8).contains(&attack));
        assert!(defense == 0 || (-12..=-8).contains(&defense));
        assert_eq!((spirit, agility, battle.enemies[0].hp), (0, 0, hp));
        both |= attack < 0 && defense < 0;
        partial |= (attack < 0) != (defense < 0);
    }
    assert!(both && partial);
}

#[test]
fn original_dragon_song_skips_defense_and_spirit_subtraction() {
    let mut skill = original_skill(14);
    assert!(skill.ignore_defense);
    assert_eq!(
        logic::skill_effect(&skill, &stats(80), &stats(900), true),
        520
    );
    skill.ignore_defense = false;
    assert_eq!(
        logic::skill_effect(&skill, &stats(80), &stats(900), true),
        408
    );
}

#[test]
fn stat_modifiers_stack_between_half_and_double_base_with_odd_value_rounding() {
    let mut battle = build_1v2();
    let mut skill = damage_skill(1, 999, vec![], vec![]);
    skill.affect_stats = [true; 4];
    battle.members[0].stats = stats(101);
    battle.enemies[0].stats = stats(101);
    for target in [Source::Party(0), Source::Enemy(0)] {
        skill.scope = 0;
        assert_eq!(battle.skill_stat_effects(target, &skill, 100, 999).len(), 4);
        assert_eq!(battle.battler_stats(target), stats(51));
        assert!(
            battle
                .skill_stat_effects(target, &skill, 100, 999)
                .is_empty()
        );
        skill.scope = 3;
        battle.skill_stat_effects(target, &skill, 100, 25);
        assert_eq!(battle.battler_stats(target), stats(76));
        battle.skill_stat_effects(target, &skill, 100, 999);
        assert_eq!(battle.battler_stats(target), stats(202));
        assert!(
            battle
                .skill_stat_effects(target, &skill, 100, 999)
                .is_empty()
        );
    }
}

#[test]
fn modified_stats_drive_physical_damage_and_the_next_round_order() {
    let mut battle = build_1v2();
    battle.members[0].stats = stats(100);
    battle.enemies[0].stats = stats(100);
    battle.members[0].weapon_hit = 100;
    battle.members[0].stat_modifiers = [100, 0, 0, 100];
    let Strike::Hit { dmg, .. } = battle.plan_strike(0, 0) else {
        panic!("guaranteed hit");
    };
    assert!((60..=90).contains(&dmg));
    battle.commit(Command::Nothing);
    assert_eq!(
        battle
            .queue
            .iter()
            .find(|action| matches!(action.source, Source::Party(0)))
            .unwrap()
            .agility,
        200
    );
    battle.new_round();
    assert_eq!(battle.members[0].stat_modifiers, [100, 0, 0, 100]);
}

#[test]
fn death_clears_stat_modifiers_and_a_weapon_kill_never_leaves_negative_hp() {
    let mut battle = build_1v2();
    battle.enemies[0].stat_modifiers = [5; 4];
    battle.land_strike(0, 9999);
    assert_eq!(battle.enemies[0].hp, 0);
    assert_eq!(battle.enemies[0].stat_modifiers, [0; 4]);
    battle.members[0].stat_modifiers = [5; 4];
    battle.hit_member(0, 9999, 0);
    assert_eq!(battle.members[0].stat_modifiers, [0; 4]);
    battle.members[0].hp = 1;
    assert_eq!(
        battle.battler_stats(Source::Party(0)).attack,
        battle.members[0].stats.attack
    );
}

#[test]
fn actor_base_stats_and_battle_stats_have_distinct_caps() {
    let mut battle = build_1v2();
    battle.members[0].stats = stats(1200);
    battle.members[0].stat_modifiers = [999; 4];
    assert_eq!(battle.battler_stats(Source::Party(0)), stats(1998));
    battle.enemies[0].stats = stats(8000);
    battle.enemies[0].stat_modifiers = [8000; 4];
    assert_eq!(battle.battler_stats(Source::Enemy(0)), stats(9999));
}
