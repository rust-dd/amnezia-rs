use super::*;
use crate::battle::resolve::tests::{damage_skill, poison_state};

#[test]
fn damage_release_and_weapon_infliction_are_planned_together_but_applied_in_message_order() {
    let mut battle = attack(false);
    let mut sleep = poison_state(2);
    sleep.restriction = 1;
    sleep.release_by_damage = 100;
    sleep.message_recovery = " recovered".into();
    let mut stun = poison_state(8);
    stun.message_enemy = " stunned".into();
    battle.states = vec![sleep, stun];
    battle.enemies[0].states = vec![(2, 0)];
    battle.enemies[0].state_ranks = vec![0; 8];
    battle.members[0].weapon_states = vec![(8, 100)];
    for frame in 1..=201 {
        let progress = update(&mut battle, Controls::default(), 0);
        assert_eq!(
            logic::has_state(&battle.enemies[0].states, 2),
            frame < 88,
            "sleep at {frame}"
        );
        assert_eq!(
            logic::has_state(&battle.enemies[0].states, 8),
            frame >= 139,
            "stun at {frame}"
        );
        assert_eq!(
            battle.log.iter().any(|line| line.ends_with(" recovered")),
            frame >= 91
        );
        assert_eq!(
            battle.log.iter().any(|line| line.ends_with(" stunned")),
            frame >= 142
        );
        assert_eq!(progress == Progress::Boundary, frame == 201);
    }
}

#[test]
fn death_state_preserves_the_original_extra_wait_but_starts_only_one_death_and_sound() {
    let mut battle = attack(false);
    let mut death = poison_state(1);
    death.message_enemy = " collapsed".into();
    battle.states = vec![death];
    battle.enemies[0].state_ranks = vec![0];
    let mut skill = damage_skill(90, 0, vec![], vec![1]);
    skill.affect_hp = false;
    skill.variance = 0;
    skill.magical_rate = 0;
    battle.skills = vec![skill];
    battle.queue[0].kind = Command::Skill {
        skill_id: 90,
        target: 0,
    };
    for frame in 1..=184 {
        let progress = update(&mut battle, Controls::default(), 0);
        assert_eq!(battle.enemies[0].hp, if frame < 63 { 1000 } else { 0 });
        assert_eq!(
            battle
                .pending_se
                .iter()
                .filter(|&&se| se == BattleSe::EnemyDefeated)
                .count(),
            usize::from(frame >= 63)
        );
        assert_eq!(
            battle
                .log
                .iter()
                .filter(|line| line.ends_with(" collapsed"))
                .count(),
            if frame < 63 {
                0
            } else if frame < 125 {
                1
            } else {
                2
            }
        );
        assert_eq!(progress == Progress::Boundary, frame == 184);
    }
}

#[test]
fn ongoing_and_recovered_condition_messages_precede_the_four_frame_usage_gap() {
    for recovered in [false, true] {
        let mut battle = attack(false);
        let mut state = poison_state(2);
        state.message_affected = " affected".into();
        state.message_recovery = " recovered".into();
        state.auto_release_prob = if recovered { 100 } else { 0 };
        battle.states = vec![state];
        battle.members[0].states = vec![(2, 1)];
        battle.queue[0].kind = Command::Nothing;
        for frame in 1..=63 {
            let progress = update(&mut battle, Controls::default(), 0);
            assert_eq!(battle.pending_action_flashes.len(), 1);
            assert_eq!(battle.log.len(), usize::from(frame >= 4));
            assert_eq!(progress == Progress::Boundary, frame == 63);
        }
        assert!(battle.log[0].ends_with(if recovered { " recovered" } else { " affected" }));
    }
}
