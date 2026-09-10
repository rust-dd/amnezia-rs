use super::*;

#[test]
fn original_periodic_state_damage_is_silent_without_a_configured_affected_message() {
    let mut battle = build_1v2();
    messages::original_text(&mut battle);
    for target in [Source::Party(0), Source::Enemy(0)] {
        *battle.battler_states_mut(target) = vec![(2, 0), (10, 0)];
        let hp = battle.battler_hp(target);
        battle.tick_state_hp(target);
        assert!(battle.battler_hp(target) < hp);
        assert!(battle.log.is_empty());
    }
}

#[test]
fn action_start_selects_one_highest_priority_state_including_just_healed_states() {
    for (poison_priority, sleep_priority, expected) in [
        (60, 50, None),
        (50, 60, Some("Ron felébred")),
        (50, 50, Some("Ron felébred")),
    ] {
        let mut battle = build_1v2();
        messages::original_text(&mut battle);
        battle.members[0].name = "Ron".into();
        battle.members[0].states = vec![(2, 0), (7, 0)];
        for state in &mut battle.states {
            state.hold_turn = 0;
            state.auto_release_prob = if state.id == 7 { 100 } else { 0 };
            state.priority = if state.id == 7 {
                sleep_priority
            } else {
                poison_priority
            };
        }
        battle.recover_before_action(Source::Party(0));
        assert_eq!(battle.members[0].states, [(2, 1)]);
        assert_eq!(battle.log, expected.into_iter().collect::<Vec<_>>());
    }
}

#[test]
fn action_start_keeps_ongoing_and_blank_recovery_messages_distinct() {
    let mut battle = build_1v2();
    messages::original_text(&mut battle);
    battle.enemies[0].name = "Bandita".into();
    battle.enemies[0].states = vec![(2, 0)];
    let poison = battle
        .states
        .iter_mut()
        .find(|state| state.id == 2)
        .unwrap();
    poison.hold_turn = 0;
    poison.auto_release_prob = 0;
    poison.message_affected = " mérgezésben szenved".into();
    poison.message_recovery.clear();
    battle.recover_before_action(Source::Enemy(0));
    assert_eq!(battle.log, ["Bandita mérgezésben szenved"]);
    battle
        .states
        .iter_mut()
        .find(|state| state.id == 2)
        .unwrap()
        .auto_release_prob = 100;
    battle.recover_before_action(Source::Enemy(0));
    assert_eq!(battle.log, ["Bandita mérgezésben szenved", "Bandita"]);
}

#[test]
fn damage_recovery_and_weapon_infliction_use_the_original_target_side_messages() {
    let mut battle = build_1v2();
    messages::original_text(&mut battle);
    battle.members[0].name = "Ron".into();
    battle.enemies[0].name = "Bandita".into();
    battle
        .states
        .iter_mut()
        .find(|state| state.id == 7)
        .unwrap()
        .release_by_damage = 100;
    battle.members[0].weapon_states = vec![(2, 100)];
    battle.members[0].state_ranks = vec![0, 0];
    battle.enemies[0].state_ranks = vec![0, 0];
    for target in [Source::Party(0), Source::Enemy(0)] {
        *battle.battler_states_mut(target) = vec![(7, 0)];
        battle.release_states_from_damage(target, 100);
        battle.weapon_states(Source::Party(0), target);
    }
    assert_eq!(
        battle.log,
        [
            "Ron felébred",
            "Ron mérgezést kap",
            "Bandita felébred",
            "Bandita megmérgeződik"
        ]
    );
}
