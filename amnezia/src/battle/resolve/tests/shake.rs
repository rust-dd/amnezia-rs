use super::*;

#[test]
fn only_positive_nonabsorbing_party_damage_requests_a_shake() {
    for target in [Source::Party(0), Source::Enemy(0)] {
        for normal in [false, true] {
            for base in [0, 1, 10] {
                for defending in [false, true] {
                    let mut battle = build_1v2();
                    match target {
                        Source::Party(i) => battle.members[i].defending = defending,
                        Source::Enemy(i) => battle.enemies[i].defending = defending,
                    }
                    let damage = battle.hit_battler(target, base, 0, 0, normal);
                    assert_eq!(
                        battle.pending_shake,
                        matches!(target, Source::Party(_)) && normal && damage > 0
                    );
                }
            }
        }
    }
}

#[test]
fn confused_ally_strikes_share_the_shake_rule_and_a_new_round_drops_stale_feedback() {
    for damage in [None, Some(0), Some(5)] {
        let mut battle = build_party2();
        battle.land_ally_strike(Source::Party(0), Source::Party(1), damage);
        assert_eq!(
            battle.pending_shake,
            damage.is_some_and(|damage| damage > 0)
        );
        battle.new_round();
        assert!(!battle.pending_shake);
    }
}
