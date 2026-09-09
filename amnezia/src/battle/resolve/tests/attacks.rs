use super::*;

#[test]
fn a_party_attack_wounds_its_target_and_logs() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    battle.commit(Command::Attack { target: 0 });
    let before = battle.enemies[0].hp;
    while battle.resolve_next() {}
    assert!(battle.enemies[0].hp < before);
    assert!(battle.log.iter().any(|l| l.contains("sebződik")));
}

#[test]
fn a_forced_critical_triples_the_blow() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    battle.members[0].weapon_crit = 100;
    let base = logic::physical_damage(
        battle.members[0].stats.attack,
        battle.enemies[0].stats.defense,
    );
    let Strike::Hit { dmg, crit } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    assert!(crit);
    // var=4 spreads the tripled base by up to ±20%; the crit clearly beats a
    // plain blow either way.
    let tripled = base * 3;
    assert!(dmg >= tripled - tripled * 2 / 10 && dmg <= tripled + tripled * 2 / 10 + 1);
    assert!(dmg > base);
}

#[test]
fn a_missed_strike_deals_no_damage() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 90;
    // The effective chance is the base hit adjusted by the agility gap; wind
    // the rng to a state whose next to-hit roll falls in that miss band.
    let hit = logic::to_hit(
        battle.members[0].weapon_hit,
        battle.members[0].stats.agility,
        battle.enemies[0].stats.agility,
    );
    loop {
        let mut probe = battle.rng;
        if (rng_next(&mut probe) % 100) as i32 >= hit {
            break;
        }
        rng_next(&mut battle.rng);
    }
    let before = battle.enemies[0].hp;
    let strike = battle.strike_enemy(0, 0);
    assert!(matches!(strike, Strike::Miss));
    assert_eq!(battle.enemies[0].hp, before);
}

#[test]
fn wiping_the_party_yields_defeat() {
    let mut battle = build_1v2();
    battle.members[0].hp = 0;
    assert!(matches!(battle.end_state(), Some(BattleOutcome::Defeat)));
}

#[test]
fn defend_halves_the_hit_a_member_takes() {
    let mut battle = build_1v2();
    battle.members[0].defending = true;
    let full = battle.members[0].hp;
    // base 8 with var=4 spreads to [7,10], halved by defence to [3,5].
    battle.hit_member(0, 8, 4, 100);
    let taken = full - battle.members[0].hp;
    assert!((3..=5).contains(&taken));
}

#[test]
fn finish_victory_records_reward_and_log() {
    let mut battle = build_1v2();
    battle.finish(BattleOutcome::Victory);
    assert!(battle.phase == Phase::Outcome);
    assert_eq!(battle.reward_gold, 60);
    assert!(battle.log_tail().contains("Győzelem"));
}

#[test]
fn a_failed_escape_raises_the_next_chance_by_ten_and_a_first_strike_is_certain() {
    let mut battle = build_1v2();
    battle.escape_chance = 0;
    assert!(!battle.attempt_escape());
    assert_eq!(battle.escape_chance, 10);
    battle.first_strike = true;
    assert!(battle.attempt_escape());
    assert_eq!(battle.escape_chance, 10);
}
