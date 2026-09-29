use super::*;
use crate::battle::model::testkit::build_party2;
use crate::battle::resolve::tests::{damage_skill, heal_skill, poison_state};

fn cast(scope: u32, animation: u32) -> Battle {
    let mut battle = build_party2();
    battle.enemies.push(build_1v2().enemies.remove(1));
    battle.log.clear();
    battle.members[0].name = "Ron".into();
    let mut skill = damage_skill(90, 10, vec![], vec![]);
    skill.scope = scope;
    skill.magical_rate = 0;
    skill.variance = 0;
    skill.animation_id = animation;
    skill.using_message1 = " casts".into();
    battle.skills = vec![skill];
    for enemy in &mut battle.enemies {
        enemy.hp = 1000;
        enemy.max_hp = 1000;
    }
    battle.queue = vec![Action {
        source: Source::Party(0),
        kind: Command::Skill {
            skill_id: 90,
            target: 0,
        },
        agility: 1,
    }];
    battle.queue_at = 0;
    battle
}

#[test]
fn skill_cost_usage_animation_feedback_and_hp_have_distinct_boundaries() {
    let mut battle = cast(0, 7);
    let sp = battle.members[0].sp;
    let rng = battle.rng;
    for frame in 1..=114 {
        let progress = update(&mut battle, Controls::default(), 20);
        assert_eq!(
            battle.members[0].sp,
            sp - if frame >= 4 { 3 } else { 0 },
            "SP at {frame}"
        );
        assert_eq!(battle.pending_anims.len(), usize::from(frame >= 4));
        assert_eq!(battle.rng == rng, frame < 63);
        assert_eq!(battle.pending_blinks.len(), usize::from(frame >= 66));
        assert_eq!(
            battle.enemies[0].hp,
            if frame < 105 { 1000 } else { 990 },
            "HP at {frame}"
        );
        assert_eq!(progress == Progress::Boundary, frame == 114);
    }
}

#[test]
fn two_usage_lines_each_wait_and_only_the_last_starts_animation() {
    let mut battle = cast(0, 7);
    battle.skills[0].using_message2 = "second line".into();
    for frame in 1..=173 {
        let progress = update(&mut battle, Controls::default(), 20);
        assert_eq!(battle.pending_anims.len(), usize::from(frame >= 63));
        assert_eq!(battle.enemies[0].hp, if frame < 164 { 1000 } else { 990 });
        assert_eq!(
            battle.log.len(),
            if frame < 4 {
                0
            } else if frame < 63 {
                1
            } else if frame < 125 {
                2
            } else {
                3
            }
        );
        assert_eq!(progress == Progress::Boundary, frame == 173);
    }
}

#[test]
fn multi_target_casts_roll_apply_and_pop_results_per_target_but_pay_and_animate_once() {
    let mut battle = cast(1, 7);
    battle.enemies[0].switch_on_after_action = None;
    let sp = battle.members[0].sp;
    let mut first_rng = 0;
    for frame in 1..=165 {
        let progress = update(&mut battle, Controls::default(), 20);
        if frame == 63 {
            first_rng = battle.rng;
        }
        if (64..114).contains(&frame) {
            assert_eq!(battle.rng, first_rng);
        }
        if frame == 114 {
            assert_ne!(battle.rng, first_rng);
        }
        assert_eq!(battle.enemies[0].hp, if frame < 105 { 1000 } else { 990 });
        assert_eq!(battle.enemies[1].hp, if frame < 156 { 1000 } else { 990 });
        assert_eq!(battle.pending_anims.len(), usize::from(frame >= 4));
        assert_eq!(battle.members[0].sp, sp - if frame >= 4 { 3 } else { 0 });
        if frame == 115 {
            assert_eq!(battle.messages.console.visible(), ["Ron casts"]);
        }
        assert_eq!(progress == Progress::Boundary, frame == 165);
    }
}

#[test]
fn a_second_recipient_killed_during_the_first_result_is_skipped_without_an_extra_roll() {
    let mut battle = cast(1, 0);
    for _ in 1..=70 {
        update(&mut battle, Controls::default(), 0);
    }
    let rng = battle.rng;
    battle.enemies[1].hp = 0;
    for frame in 71..=114 {
        let progress = update(&mut battle, Controls::default(), 0);
        assert_eq!(battle.rng, rng);
        assert_eq!(progress == Progress::Boundary, frame == 114);
    }
    assert_eq!(battle.hit_reports.len(), 1);
}

#[test]
fn party_sound_only_animations_cap_the_usage_hold_at_forty_frames() {
    for (fast, execution) in [(false, 63), (true, 43)] {
        let mut battle = cast(3, 7);
        battle.members[0].hp = 1;
        for frame in 1..=execution {
            update(&mut battle, Controls { fast, hold: false }, 120);
            assert_eq!(battle.members[0].hp, if frame < execution { 1 } else { 11 });
        }
        assert!(battle.pending_anims[0].sound_only);
    }
}

#[test]
fn healing_applies_before_its_message_but_revival_waits_for_the_death_cure() {
    let mut battle = cast(3, 0);
    battle.members[0].hp = 1;
    for frame in 1..=125 {
        let progress = update(&mut battle, Controls::default(), 0);
        assert_eq!(battle.members[0].hp, if frame < 63 { 1 } else { 11 });
        assert_eq!(
            battle.log.len(),
            if frame < 4 {
                0
            } else if frame < 66 {
                1
            } else {
                2
            }
        );
        assert_eq!(progress == Progress::Boundary, frame == 125);
    }

    let mut battle = cast(3, 0);
    let mut skill = heal_skill(90, 10);
    skill.magical_rate = 0;
    skill.variance = 0;
    skill.affected_states = vec![1];
    battle.skills = vec![skill];
    let mut dead = poison_state(1);
    dead.message_recovery = " revived".into();
    battle.states = vec![dead];
    battle.members[1].hp = 0;
    battle.members[1].states = vec![(1, 0)];
    battle.queue[0].kind = Command::Skill {
        skill_id: 90,
        target: 1,
    };
    for frame in 1..=125 {
        let progress = update(&mut battle, Controls::default(), 0);
        assert_eq!(battle.members[1].hp, if frame < 63 { 0 } else { 10 });
        assert_eq!(logic::has_state(&battle.members[1].states, 1), frame < 63);
        assert_eq!(progress == Progress::Boundary, frame == 125);
    }
    assert!(battle.log.last().unwrap().ends_with(" revived"));
}

#[test]
fn unavailable_inventory_cancels_before_flash_and_consumption_occurs_at_usage_only() {
    for available in [false, true] {
        let mut battle = cast(0, 0);
        battle.items = vec![crate::battle::resolve::tests::medicine(90, 10, 0, vec![])];
        battle.queue[0].kind = Command::Item {
            item_id: 90,
            target: 0,
        };
        battle.members[0].hp = 1;
        let mut calls = Vec::new();
        for frame in 1..=4 {
            let progress = battle.advance_action(
                Controls::default(),
                |_| 0,
                |id, consume| {
                    calls.push((frame, id, consume));
                    available
                },
            );
            if !available {
                assert_eq!(
                    progress,
                    if frame == 1 {
                        Progress::Boundary
                    } else {
                        Progress::Done
                    }
                );
                assert!(battle.pending_action_flashes.is_empty());
            }
        }
        assert_eq!(
            calls,
            if available {
                vec![(1, 90, false), (4, 90, true)]
            } else {
                vec![(1, 90, false)]
            }
        );
        assert_eq!(
            battle
                .pending_se
                .iter()
                .filter(|&&se| se == BattleSe::UseItem)
                .count(),
            usize::from(available)
        );
    }
}

#[test]
fn enemy_single_ally_casts_recheck_the_original_recipient_before_spending_or_flashing() {
    for dead in [false, true] {
        let mut battle = cast(3, 0);
        battle.skills[0].affect_hp = dead;
        battle.skills[0].affected_states = vec![2];
        battle.states = vec![poison_state(2)];
        battle.enemies[1].hp = if dead { 0 } else { 1000 };
        battle.enemies[0].hp = 1;
        battle.enemies[0].sp = 3;
        battle.enemies[0].switch_on_after_action = Some(545);
        battle.queue[0] = Action {
            source: Source::Enemy(0),
            kind: Command::Skill {
                skill_id: 90,
                target: 1,
            },
            agility: 1,
        };
        assert_eq!(
            update(&mut battle, Controls::default(), 0),
            Progress::Boundary
        );
        assert_eq!(battle.enemies[0].hp, 1);
        assert_eq!(battle.enemies[0].sp, 3);
        assert!(battle.pending_action_flashes.is_empty());
        assert!(battle.pending_switches.is_empty());
        assert!(battle.log.is_empty());
    }
}
