use super::*;

#[test]
fn an_ally_heal_raises_a_wounded_ally_clamped_to_max() {
    let mut battle = build_1v2();
    battle.skills = vec![heal_skill(2, 40)];
    let max = battle.members[0].max_hp;
    battle.members[0].hp = max - 3;
    battle.cast_skill(0, 2, 0);
    assert_eq!(battle.members[0].hp, max);
}

#[test]
fn a_recover_hp_item_raises_the_users_hp_clamped_to_max() {
    let mut battle = build_1v2();
    let max = battle.members[0].max_hp;
    battle.items = vec![medicine(50, 20, 0, vec![])];
    battle.members[0].hp = 10;
    let line = battle.apply_item(0, 50, 0);
    assert_eq!(battle.members[0].hp, 30);
    assert!(line.contains("+20 HP"));
    battle.members[0].hp = max - 5;
    battle.items = vec![medicine(51, 200, 0, vec![])];
    battle.apply_item(0, 51, 0);
    assert_eq!(battle.members[0].hp, max);
}

#[test]
fn an_item_used_on_an_ally_heals_that_member_not_the_caster() {
    let mut battle = build_party2();
    battle.items = vec![medicine(50, 20, 0, vec![])];
    let max = battle.members[1].max_hp;
    battle.members[1].hp = (max - 25).max(0);
    let before_ally = battle.members[1].hp;
    let before_caster = battle.members[0].hp;
    let line = battle.apply_item(0, 50, 1);
    assert_eq!(battle.members[1].hp, (before_ally + 20).min(max));
    assert_eq!(battle.members[0].hp, before_caster);
    assert!(line.contains("+20 HP"));
}

#[test]
fn a_skill_can_miss_its_to_hit_roll_and_deal_nothing() {
    let mut battle = build_1v2();
    let mut s = damage_skill(1, 30, vec![], vec![]);
    s.hit = 50;
    battle.skills = vec![s];
    // Wind the rng so the skill's first (to-hit) draw lands in the miss band.
    let hit = logic::skill_to_hit(
        &battle.skills[0],
        battle.members[0].stats.agility,
        battle.enemies[0].stats.agility,
        true,
    );
    loop {
        let mut probe = battle.rng;
        if (rng_next(&mut probe) % 100) as i32 >= hit {
            break;
        }
        rng_next(&mut battle.rng);
    }
    let skill = battle.skills[0].clone();
    let before = battle.enemies[0].hp;
    let lines = battle.skill_hit_enemy(0, 0, &skill);
    assert_eq!(
        battle.enemies[0].hp, before,
        "a missed skill deals no damage"
    );
    assert!(lines.iter().any(|l| l.contains("elkerülte")));
}
