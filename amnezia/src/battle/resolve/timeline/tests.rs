use super::*;
use crate::battle::model::testkit::build_1v2;
mod casts;
mod states;

fn attack(critical: bool) -> Battle {
    let mut battle = build_1v2();
    battle.log.clear();
    battle.members[0].weapon_hit = 100;
    battle.members[0].base_critical_denominator = None;
    battle.members[0].weapon_crit = if critical { 100 } else { 0 };
    battle.members[0].attack_animation = 0;
    battle.enemies[0].hp = 1000;
    battle.enemies[0].max_hp = 1000;
    battle.queue = vec![Action {
        source: Source::Party(0),
        kind: Command::Attack { target: 0 },
        agility: 1,
    }];
    battle.queue_at = 0;
    battle.phase = Phase::Resolve;
    battle
}

fn update(battle: &mut Battle, controls: Controls, animation: u32) -> Progress {
    battle.messages.console.update();
    battle.advance_deaths(1.0 / 60.0);
    battle.advance_action(controls, |_| animation, |_, _| true)
}

#[test]
fn physical_action_waits_and_hp_application_follow_the_original_update_boundaries() {
    for (fast, execute, damage, hp, end) in [(false, 43, 49, 88, 97), (true, 23, 29, 48, 48)] {
        let mut battle = attack(false);
        let rng = battle.rng;
        for frame in 1..=end {
            let progress = update(&mut battle, Controls { fast, hold: false }, 0);
            assert_eq!(battle.members[0].sp, 37);
            assert_eq!(battle.rng == rng, frame < execute, "RNG at {frame}");
            assert_eq!(battle.enemies[0].hp == 1000, frame < hp, "HP at {frame}");
            assert_eq!(battle.pending_blinks.len(), usize::from(frame >= damage));
            assert_eq!(battle.pending_se.len(), usize::from(frame >= damage));
            assert_eq!(
                progress,
                if frame == end {
                    Progress::Boundary
                } else {
                    Progress::Waiting
                }
            );
            let lines = battle.messages.console.visible().len();
            assert_eq!(
                lines,
                if frame <= 4 {
                    0
                } else if frame <= damage {
                    1
                } else {
                    2
                }
            );
        }
        assert!(battle.messages.console.len() == 0);
        assert!(!battle.action_in_progress());
    }
}

#[test]
fn a_critical_has_its_own_hold_before_damage_and_does_not_mutate_hp_early() {
    let mut battle = attack(true);
    for frame in 1..=126 {
        let progress = update(&mut battle, Controls::default(), 0);
        assert_eq!(battle.enemies[0].hp == 1000, frame < 117, "HP at {frame}");
        assert_eq!(battle.pending_se.len(), usize::from(frame >= 78));
        assert_eq!(
            battle.log.len(),
            if frame < 4 {
                0
            } else if frame < 46 {
                1
            } else if frame < 78 {
                2
            } else {
                3
            }
        );
        assert_eq!(progress == Progress::Boundary, frame == 126);
    }
}

#[test]
fn cancel_freezes_action_waits_without_spending_rng_or_replaying_feedback() {
    let mut battle = attack(false);
    for _ in 0..10 {
        update(&mut battle, Controls::default(), 0);
    }
    let rng = battle.rng;
    let log = battle.log.clone();
    for _ in 0..90 {
        assert_eq!(
            update(
                &mut battle,
                Controls {
                    fast: true,
                    hold: true
                },
                0
            ),
            Progress::Waiting
        );
        assert_eq!(battle.rng, rng);
        assert_eq!(battle.log, log);
        assert_eq!(battle.enemies[0].hp, 1000);
        assert!(battle.pending_se.is_empty());
    }
    for frame in 11..=97 {
        assert_eq!(
            update(&mut battle, Controls::default(), 0) == Progress::Boundary,
            frame == 97
        );
    }
}

#[test]
fn the_usage_wait_includes_animation_duration_before_the_first_damage_roll() {
    for fast in [false, true] {
        let mut battle = attack(false);
        battle.members[0].attack_animation = 9;
        let rng = battle.rng;
        for frame in 1..=75 {
            update(&mut battle, Controls { fast, hold: false }, 72);
            assert_eq!(battle.rng == rng, frame < 75);
            assert_eq!(battle.enemies[0].hp, 1000);
        }
        assert_eq!(battle.pending_anims.len(), 1);
        assert_eq!(battle.pending_anims[0].anim_id, 9);
    }
}

#[test]
fn death_begins_after_the_damage_message_and_has_one_kill_sound() {
    let mut battle = attack(false);
    battle.enemies[0].hp = 1;
    for frame in 1..=156 {
        let progress = update(&mut battle, Controls::default(), 0);
        assert_eq!(battle.enemies[0].hp, i32::from(frame < 88));
        assert_eq!(battle.enemies[0].dying.is_some(), frame >= 88);
        assert_eq!(
            battle
                .pending_se
                .iter()
                .filter(|&&se| se == BattleSe::EnemyDefeated)
                .count(),
            usize::from(frame >= 88)
        );
        assert_eq!(progress == Progress::Boundary, frame == 156);
    }
}
