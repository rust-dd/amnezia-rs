use super::*;

#[test]
fn a_party_strike_queues_the_weapon_animation_on_the_struck_foe() {
    let mut battle = build_weapon_anim(7);
    battle.commit(Command::Attack { target: 0 });
    while battle.resolve_next() {}
    let anim = battle
        .pending_anims
        .iter()
        .find(|a| a.anim_id == 7)
        .expect("the weapon attack animation should be queued");
    assert_eq!(anim.targets.len(), 1, "one target for the struck foe");
    let (x, y) = anim.targets[0];
    assert!((x + 60.0).abs() < 1e-6, "x = {x}");
    assert!((y + 20.0).abs() < 1e-6, "y = {y}");
}

#[test]
fn an_unarmed_party_strike_falls_back_to_the_actor_unarmed_animation() {
    use crate::battle::model::testkit;
    use crate::progression::Progression;
    use crate::vitals::Vitals;
    let mut ron = testkit::actor(1, 2, 63, 37);
    ron.weapon = 0;
    ron.unarmed_animation = 3;
    let actors = vec![&ron];
    let monsters = vec![testkit::monster(1, 30, 10, 30)];
    let troop = testkit::troop(&[(1, 100, 100)]);
    let mut battle = Battle::build(
        &troop,
        &monsters,
        &actors,
        &[testkit::slots(&ron)],
        &[],
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        1,
    );
    battle.commit(Command::Attack { target: 0 });
    while battle.resolve_next() {}
    assert!(
        battle.pending_anims.iter().any(|a| a.anim_id == 3),
        "the unarmed attacker should queue its unarmed_animation"
    );
}

#[test]
fn a_zero_animation_attacker_queues_nothing() {
    // build_1v2's hero is bare-handed with unarmed_animation 0, so its swing
    // queues no member animation, and rpg2k enemy normal attacks play none
    // either — so a full round queues nothing, and id 0 is never pushed.
    let mut battle = build_1v2();
    battle.commit(Command::Attack { target: 0 });
    while battle.resolve_next() {}
    assert!(battle.pending_anims.iter().all(|a| a.anim_id != 0));
}

#[test]
fn an_enemy_normal_attack_queues_no_animation() {
    // An rpg2k enemy normal attack shows no animation, so a strike queues
    // nothing regardless of whether the to-hit roll lands.
    let mut battle = build_1v2();
    battle.enemy_strike_member(0, 0);
    assert!(
        battle.pending_anims.is_empty(),
        "an enemy normal attack plays no animation"
    );
}

#[test]
fn new_round_clears_the_pending_animation_queue() {
    let mut battle = build_1v2();
    battle.pending_anims.push(PendingAnim {
        anim_id: 5,
        targets: vec![(1.0, 2.0)],
    });
    battle.new_round();
    assert!(battle.pending_anims.is_empty());
}

#[test]
fn a_single_target_skill_queues_one_animation_at_the_targeted_foe() {
    let mut battle = build_1v2();
    let mut s = damage_skill(1, 20, vec![], vec![]);
    s.animation_id = 9;
    battle.skills = vec![s];
    battle.cast_skill(0, 1, 0);
    let hits: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 9)
        .collect();
    assert_eq!(hits.len(), 1, "one animation for the one struck foe");
    assert_eq!(hits[0].targets.len(), 1, "one target");
    let (x, y) = hits[0].targets[0];
    assert!((x + 60.0).abs() < 1e-6, "x = {x}");
    assert!((y + 20.0).abs() < 1e-6, "y = {y}");
}

#[test]
fn an_all_enemy_skill_queues_one_animation_over_every_living_foe() {
    let mut battle = build_1v2();
    let mut s = damage_skill(1, 20, vec![], vec![]);
    s.scope = 1;
    s.animation_id = 8;
    battle.skills = vec![s];
    battle.cast_skill(0, 1, 0);
    // One queued animation for the whole cast (its SE fires once), carrying
    // every living foe as a target so its cells and flashes reach each.
    let hits: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 8)
        .collect();
    assert_eq!(hits.len(), 1, "one animation for the multi-target cast");
    let xs: Vec<f32> = hits[0].targets.iter().map(|&(x, _)| x).collect();
    assert_eq!(xs.len(), 2, "one target per living foe");
    assert!(xs.iter().any(|x| (x + 60.0).abs() < 1e-6), "foe at x=100");
    assert!(xs.iter().any(|x| (x - 40.0).abs() < 1e-6), "foe at x=200");
}

#[test]
fn an_all_ally_heal_queues_one_animation_per_living_member_at_the_party_area() {
    let mut battle = build_party2();
    let mut s = heal_skill(2, 30);
    s.scope = 4;
    s.animation_id = 5;
    battle.skills = vec![s];
    battle.cast_skill(0, 2, 0);
    let heals: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 5)
        .collect();
    assert_eq!(heals.len(), 1, "one animation for the multi-target heal");
    let slots = &heals[0].targets;
    assert_eq!(slots.len(), 2, "one target per living ally");
    assert!(
        slots.iter().all(|&(_, y)| (y - 80.0).abs() < 1e-6),
        "each plays at the party-area y"
    );
    assert!(
        (slots[0].0 - slots[1].0).abs() > 1e-6,
        "the two members' slots are spread apart"
    );
}

#[test]
fn a_zero_animation_skill_queues_nothing() {
    let mut battle = build_1v2();
    battle.skills = vec![damage_skill(1, 20, vec![], vec![])];
    battle.cast_skill(0, 1, 0);
    assert!(
        battle.pending_anims.is_empty(),
        "a 0 animation id queues nothing"
    );
}

#[test]
fn an_enemy_damage_cast_queues_the_animation_at_the_targeted_member_slot() {
    let mut battle = build_1v2();
    let mut s = damage_skill(1, 30, vec![], vec![]);
    s.animation_id = 6;
    battle.skills = vec![s];
    battle.enemy_cast(0, 1, 0);
    let hits: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 6)
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].targets.len(), 1);
    let (x, y) = hits[0].targets[0];
    assert!(x.abs() < 1e-6, "x = {x}");
    assert!((y - 80.0).abs() < 1e-6, "y = {y}");
}

#[test]
fn an_enemy_ally_scope_cast_queues_the_animation_on_the_casting_foe() {
    let mut battle = build_1v2();
    let mut s = heal_skill(2, 40);
    s.animation_id = 4;
    battle.skills = vec![s];
    battle.enemy_cast(0, 2, 0);
    let hits: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 4)
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].targets.len(), 1);
    let (x, y) = hits[0].targets[0];
    assert!((x + 60.0).abs() < 1e-6, "x = {x}");
    assert!((y + 20.0).abs() < 1e-6, "y = {y}");
}

#[test]
fn a_landed_strike_reports_the_damage_at_the_foe() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    let Strike::Hit { dmg, .. } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    let pos = battle.foe_anim_pos(0);
    let (text, kind) = damage_report(dmg);
    let number = battle
        .hit_reports
        .last()
        .expect("a landed hit has a diagnostic report");
    assert_eq!(number.pos, pos);
    assert_eq!(number.text, text);
    assert!(number.kind == kind);
}

#[test]
fn a_heal_reports_its_result() {
    let mut battle = build_1v2();
    battle.skills = vec![heal_skill(2, 40)];
    battle.members[0].hp = 10;
    battle.cast_skill(0, 2, 0);
    assert!(
        battle.hit_reports.iter().any(|n| n.kind == HitKind::Heal),
        "a heal has a diagnostic report"
    );
}

#[test]
fn a_missed_strike_reports_a_miss() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 90;
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
    assert!(matches!(battle.strike_enemy(0, 0), Strike::Miss));
    let number = battle
        .hit_reports
        .last()
        .expect("a miss has a diagnostic report");
    assert_eq!(number.text, "Miss");
    assert!(number.kind == HitKind::Miss);
}

#[test]
fn a_foe_hit_enqueues_a_guaranteed_blink_without_any_animation_flash() {
    // build_1v2's hero is bare-handed (unarmed_animation 0), so its swing plays
    // no animation and carries no flash timing — yet the struck foe blinks.
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    assert!(battle.pending_blinks.is_empty());
    battle.strike_enemy(0, 0);
    let pos = battle.foe_anim_pos(0);
    assert_eq!(battle.pending_blinks.first().copied(), Some(pos));
}

#[test]
fn felling_a_foe_starts_a_death_that_holds_resolve_then_clears() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    battle.enemies[0].hp = 1;
    battle.strike_enemy(0, 0);
    assert!(!battle.enemies[0].alive());
    assert!(battle.enemies[0].dying.is_some());
    assert!(battle.death_in_progress());
    battle.advance_deaths(DEATH_SECS + 0.1);
    assert!(!battle.death_in_progress());
    assert!(!battle.enemies[0].alive());
}

#[test]
fn self_destruct_starts_an_explosion_death_out() {
    let mut battle = build_party2();
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::SelfDestruct,
        agility: 0,
    });
    let dying = battle.enemies[0]
        .dying
        .as_ref()
        .expect("a self-destruct explodes");
    assert!(dying.explode);
    assert!(battle.death_in_progress());
}

#[test]
fn a_multi_target_cast_staggers_its_hits_across_ticks() {
    let mut battle = build_1v2();
    let mut s = damage_skill(1, 20, vec![], vec![]);
    s.scope = 1;
    battle.skills = vec![s];
    // The foes survive the cast (so no death-hold) and only defend, so the sole
    // hit reports each tick come from the staggered cast, not enemy attacks.
    for e in &mut battle.enemies {
        e.hp = 500;
        e.actions = vec![enemy_action_def(2)];
    }
    battle.commit(Command::Skill {
        skill_id: 1,
        target: 0,
    });
    let mut per_tick: Vec<usize> = Vec::new();
    let mut prev = battle.hit_reports.len();
    while battle.resolve_next() {
        let now = battle.hit_reports.len();
        per_tick.push(now - prev);
        prev = now;
    }
    let ticks_with_a_hit = per_tick.iter().filter(|&&c| c > 0).count();
    assert!(
        ticks_with_a_hit >= 2,
        "an all-enemy cast should stagger its hits across ticks, got {per_tick:?}"
    );
    assert_eq!(
        per_tick.iter().sum::<usize>(),
        2,
        "exactly the two foe hits landed, one per tick"
    );
}

#[test]
fn a_critical_announces_on_its_own_line_before_the_damage_line() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    battle.members[0].weapon_crit = 100;
    // Foes survive and only defend, so no enemy lines crowd the log.
    for e in &mut battle.enemies {
        e.hp = 500;
        e.actions = vec![enemy_action_def(2)];
    }
    battle.commit(Command::Attack { target: 0 });
    while battle.resolve_next() {}
    assert!(
        battle.log.iter().any(|l| l == "Kritikus ütés!"),
        "a critical announces on its own line, log: {:?}",
        battle.log
    );
    let damage_line = battle
        .log
        .iter()
        .find(|l| l.contains("sebződik"))
        .expect("the critical's damage line");
    assert!(
        !damage_line.contains("Kritikus"),
        "the damage line is a separate beat: {damage_line}"
    );
}

#[test]
fn a_landed_foe_hit_enqueues_the_enemy_damaged_se() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    assert!(matches!(battle.strike_enemy(0, 0), Strike::Hit { .. }));
    assert!(
        battle.pending_se.contains(&BattleSe::EnemyDamaged),
        "a landed blow queues the enemy-damaged SE: {:?}",
        battle.pending_se
    );
    // A 30-HP foe survives one blow, so no kill SE yet.
    assert!(!battle.pending_se.contains(&BattleSe::EnemyDefeated));
}

#[test]
fn felling_a_foe_enqueues_the_enemy_defeated_se() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    battle.enemies[0].hp = 1;
    battle.strike_enemy(0, 0);
    assert!(!battle.enemies[0].alive());
    assert!(
        battle.pending_se.contains(&BattleSe::EnemyDefeated),
        "felling a foe queues the kill SE: {:?}",
        battle.pending_se
    );
    assert!(battle.pending_se.contains(&BattleSe::EnemyDamaged));
}

#[test]
fn a_missed_strike_enqueues_the_dodge_se() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 90;
    let hit = logic::to_hit(
        battle.members[0].weapon_hit,
        battle.members[0].stats.agility,
        battle.enemies[0].stats.agility,
    );
    // Wind the rng to a state whose next to-hit roll lands in the miss band.
    loop {
        let mut probe = battle.rng;
        if (rng_next(&mut probe) % 100) as i32 >= hit {
            break;
        }
        rng_next(&mut battle.rng);
    }
    assert!(matches!(battle.strike_enemy(0, 0), Strike::Miss));
    assert!(
        battle.pending_se.contains(&BattleSe::Dodge),
        "an evaded blow queues the dodge SE: {:?}",
        battle.pending_se
    );
    // A pure miss lands nothing, so no damage SE.
    assert!(!battle.pending_se.contains(&BattleSe::EnemyDamaged));
}

#[test]
fn an_animated_strike_defers_its_hit_report_until_after_the_animation() {
    // A member wielding a weapon whose attack animation is 7: RM2000 plays the
    // swing, waits for it, and only then shows the damage.
    let mut battle = build_weapon_anim(7);
    battle.apply(Action {
        source: Source::Party(0),
        kind: Command::Attack { target: 0 },
        agility: 0,
    });
    assert!(
        battle.pending_anims.iter().any(|a| a.anim_id == 7),
        "the attack animation is queued when the action begins"
    );
    assert!(
        battle.hit_reports.is_empty(),
        "no hit is reported on the same tick as the animation"
    );
    assert!(
        battle.anim_hold_active(),
        "resolution holds while the swing plays"
    );
    assert!(
        matches!(battle.steps.front(), Some(Step::StrikeImpact { .. })),
        "the impact is queued as a deferred step: {:?}",
        battle.steps.front().is_some()
    );
    battle.resolve_next();
    assert!(
        !battle.hit_reports.is_empty(),
        "the hit is reported only once the deferred impact resolves"
    );
}

#[test]
fn a_zero_animation_strike_applies_immediately_without_holding() {
    // build_1v2's hero is bare-handed (unarmed_animation 0), so there is no swing
    // to wait for: the impact must land at once with no hold (never wedging).
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    let before = battle.enemies[0].hp;
    battle.apply(Action {
        source: Source::Party(0),
        kind: Command::Attack { target: 0 },
        agility: 0,
    });
    assert!(
        !battle.anim_hold_active(),
        "a 0-animation strike never holds"
    );
    assert!(battle.enemies[0].hp < before, "the blow lands on this tick");
    assert!(
        !battle.hit_reports.is_empty(),
        "its hit is reported immediately"
    );
}

#[test]
fn an_enemy_normal_attack_applies_and_reports_damage_without_holding() {
    // An rpg2k enemy normal attack plays no animation, so it applies immediately
    // (paced only by the step timer), queues no animation, and never holds.
    let mut battle = build_1v2();
    wind_enemy_hits(&mut battle, &[0]);
    let before = battle.members[0].hp;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Attack { target: 0 },
        agility: 0,
    });
    assert!(
        !battle.anim_hold_active(),
        "an enemy normal attack plays no animation, so no hold"
    );
    assert!(
        battle.members[0].hp < before,
        "the member takes the hit at once"
    );
    assert!(
        battle.pending_anims.is_empty(),
        "no animation is queued for an enemy normal attack"
    );
    assert!(
        !battle.hit_reports.is_empty(),
        "the hit is reported on this tick"
    );
}

#[test]
fn the_animation_hold_waits_for_the_animation_to_appear_then_finish() {
    let mut battle = build_1v2();
    battle.begin_anim_hold();
    assert!(battle.anim_hold_active());
    // The `LiveAnimation` is spawned by `drain_pending_anims`, which runs after
    // `resolve_tick`, so it isn't live on the first hold frame — the hold must
    // persist through that spawn lag rather than clear on the first empty reading.
    assert!(
        battle.tick_anim_hold(false),
        "holds through the one-tick spawn lag"
    );
    assert!(battle.tick_anim_hold(true), "holds while it plays");
    assert!(battle.tick_anim_hold(true));
    assert!(
        !battle.tick_anim_hold(false),
        "releases once the seen animation has finished"
    );
    assert!(!battle.anim_hold_active());
}

#[test]
fn the_animation_hold_gives_up_if_the_animation_never_appears() {
    // An unknown animation id spawns no `LiveAnimation`; the grace window must
    // bound the wait so resolution can never wedge.
    let mut battle = build_1v2();
    battle.begin_anim_hold();
    let mut ticks = 0;
    while battle.tick_anim_hold(false) {
        ticks += 1;
        assert!(
            ticks < 1000,
            "the hold must not wedge on a missing animation"
        );
    }
    assert!(!battle.anim_hold_active());
    assert!(ticks > 0, "it holds briefly before giving up");
}
