use super::*;

#[test]
fn a_party_strike_queues_the_weapon_animation_on_the_struck_foe() {
    let mut battle = build_weapon_anim(7);
    battle.commit(Command::Attack { target: 0 });
    while battle.resolve_next() {}
    // The member's swing queued its weapon animation (7) at the foe's screen
    // slot: x = foe.x - 160, y = foe.y - 120 (foe at 100, 100).
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
    ron.weapon = 0; // bare-handed
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
    let mut battle = build_1v2(); // foes at (100, 100) and (200, 100)
    let mut s = damage_skill(1, 20, vec![], vec![]);
    s.animation_id = 9;
    battle.skills = vec![s];
    battle.cast_skill(0, 1, 0); // scope 0 -> the targeted foe (index 0)
    let hits: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 9)
        .collect();
    assert_eq!(hits.len(), 1, "one animation for the one struck foe");
    assert_eq!(hits[0].targets.len(), 1, "one target");
    // foe 0 at (100, 100): x = 100 - 160 = -60, y = 100 - 120 = -20.
    let (x, y) = hits[0].targets[0];
    assert!((x + 60.0).abs() < 1e-6, "x = {x}");
    assert!((y + 20.0).abs() < 1e-6, "y = {y}");
}

#[test]
fn an_all_enemy_skill_queues_one_animation_over_every_living_foe() {
    let mut battle = build_1v2(); // two living foes, at x = 100 and x = 200
    let mut s = damage_skill(1, 20, vec![], vec![]);
    s.scope = 1; // all enemies
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
    let mut battle = build_party2(); // two living members
    let mut s = heal_skill(2, 30);
    s.scope = 4; // all allies
    s.animation_id = 5;
    battle.skills = vec![s];
    battle.cast_skill(0, 2, 0);
    // One queued animation for the whole cast, carrying every living ally.
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
    // damage_skill leaves animation_id at its 0 default.
    battle.skills = vec![damage_skill(1, 20, vec![], vec![])];
    battle.cast_skill(0, 1, 0);
    assert!(
        battle.pending_anims.is_empty(),
        "a 0 animation id queues nothing"
    );
}

#[test]
fn an_enemy_damage_cast_queues_the_animation_at_the_targeted_member_slot() {
    let mut battle = build_1v2(); // one member
    let mut s = damage_skill(1, 30, vec![], vec![]); // scope 0 -> hits a member
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
    // The lone member's party slot: centred x, party-area y.
    let (x, y) = hits[0].targets[0];
    assert!(x.abs() < 1e-6, "x = {x}");
    assert!((y - 80.0).abs() < 1e-6, "y = {y}");
}

#[test]
fn an_enemy_ally_scope_cast_queues_the_animation_on_the_casting_foe() {
    let mut battle = build_1v2(); // caster foe 0 at (100, 100)
    let mut s = heal_skill(2, 40); // scope 3 -> the foe heals itself
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
fn a_landed_strike_pops_a_damage_number_at_the_foe() {
    let mut battle = build_1v2(); // foe 0 at (100, 100)
    battle.members[0].weapon_hit = 100; // never miss
    let Strike::Hit { dmg, .. } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    let pos = battle.foe_anim_pos(0);
    let (text, kind) = number_for(dmg);
    let number = battle
        .pending_numbers
        .last()
        .expect("a landed hit pops a floating number");
    assert_eq!(number.pos, pos);
    assert_eq!(number.text, text);
    assert!(number.kind == kind);
}

#[test]
fn a_heal_pops_a_heal_coloured_number() {
    let mut battle = build_1v2();
    battle.skills = vec![heal_skill(2, 40)]; // scope 3 HP heal
    battle.members[0].hp = 10;
    battle.cast_skill(0, 2, 0);
    assert!(
        battle
            .pending_numbers
            .iter()
            .any(|n| n.kind == NumberKind::Heal),
        "a heal enqueues a heal-coloured number"
    );
}

#[test]
fn a_missed_strike_pops_a_miss_number() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 90;
    let hit = logic::to_hit(
        logic::effective_hit(battle.members[0].weapon_hit),
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
    let number = battle.pending_numbers.last().expect("a miss pops a number");
    assert_eq!(number.text, "Miss");
    assert!(number.kind == NumberKind::Miss);
}

#[test]
fn a_foe_hit_enqueues_a_guaranteed_blink_without_any_animation_flash() {
    // build_1v2's hero is bare-handed (unarmed_animation 0), so its swing plays
    // no animation and carries no flash timing — yet the struck foe blinks.
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    assert!(battle.pending_blinks.is_empty());
    battle.strike_enemy(0, 0);
    let pos = battle.foe_anim_pos(0);
    assert_eq!(battle.pending_blinks.first().copied(), Some(pos));
}

#[test]
fn felling_a_foe_starts_a_death_that_holds_resolve_then_clears() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    battle.enemies[0].hp = 1; // one blow from death
    battle.strike_enemy(0, 0);
    assert!(!battle.enemies[0].alive());
    assert!(battle.enemies[0].dying.is_some());
    assert!(battle.death_in_progress()); // resolve_tick holds while this is true
    battle.advance_deaths(DEATH_SECS + 0.1); // let the beat play out
    assert!(!battle.death_in_progress()); // hold released
    assert!(!battle.enemies[0].alive()); // and the foe is gone for good
}

#[test]
fn self_destruct_starts_an_explosion_death_out() {
    let mut battle = build_party2(); // one foe
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
fn a_multi_target_cast_staggers_its_damage_numbers_across_ticks() {
    let mut battle = build_1v2(); // two foes
    let mut s = damage_skill(1, 20, vec![], vec![]);
    s.scope = 1; // all enemies
    battle.skills = vec![s];
    // The foes survive the cast (so no death-hold) and only defend, so the sole
    // damage numbers each tick come from the staggered cast, not enemy attacks.
    for e in &mut battle.enemies {
        e.hp = 500;
        e.actions = vec![enemy_action_def(2)]; // basic 2 = defend
    }
    battle.commit(Command::Skill {
        skill_id: 1,
        target: 0,
    });
    // Drive resolution tick by tick, counting how many floating numbers land
    // each tick. The all-enemy cast must spread its two numbers over two ticks.
    let mut per_tick: Vec<usize> = Vec::new();
    let mut prev = battle.pending_numbers.len();
    while battle.resolve_next() {
        let now = battle.pending_numbers.len();
        per_tick.push(now - prev);
        prev = now;
    }
    let ticks_with_a_number = per_tick.iter().filter(|&&c| c > 0).count();
    assert!(
        ticks_with_a_number >= 2,
        "an all-enemy cast should stagger its numbers across ticks, got {per_tick:?}"
    );
    assert_eq!(
        per_tick.iter().sum::<usize>(),
        2,
        "exactly the two foe damage numbers landed, one per tick"
    );
}

#[test]
fn a_critical_announces_on_its_own_line_before_the_damage_line() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    battle.members[0].weapon_crit = 100; // always crit
    // Foes survive and only defend, so no enemy lines crowd the log.
    for e in &mut battle.enemies {
        e.hp = 500;
        e.actions = vec![enemy_action_def(2)];
    }
    battle.commit(Command::Attack { target: 0 });
    while battle.resolve_next() {}
    // The critical term is emitted as its own distinct log line.
    assert!(
        battle.log.iter().any(|l| l == "Kritikus ütés!"),
        "a critical announces on its own line, log: {:?}",
        battle.log
    );
    // The damage line follows separately and no longer folds in the crit word.
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
    battle.members[0].weapon_hit = 100; // never miss
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
    battle.members[0].weapon_hit = 100; // never miss
    battle.enemies[0].hp = 1; // one blow from death
    battle.strike_enemy(0, 0);
    assert!(!battle.enemies[0].alive());
    assert!(
        battle.pending_se.contains(&BattleSe::EnemyDefeated),
        "felling a foe queues the kill SE: {:?}",
        battle.pending_se
    );
    // The damage SE still fires for the killing blow itself.
    assert!(battle.pending_se.contains(&BattleSe::EnemyDamaged));
}

#[test]
fn a_missed_strike_enqueues_the_dodge_se() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 90; // bare-hands default
    let hit = logic::to_hit(
        logic::effective_hit(battle.members[0].weapon_hit),
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
fn an_animated_strike_defers_its_damage_number_until_after_the_animation() {
    // A member wielding a weapon whose attack animation is 7: RM2000 plays the
    // swing, waits for it, and only then shows the damage.
    let mut battle = build_weapon_anim(7);
    battle.apply(Action {
        source: Source::Party(0),
        kind: Command::Attack { target: 0 },
        agility: 0,
    });
    // The swing animation is queued now, up front...
    assert!(
        battle.pending_anims.iter().any(|a| a.anim_id == 7),
        "the attack animation is queued when the action begins"
    );
    // ...but no damage number lands on the same tick as the animation — it is
    // held behind the animation as a deferred impact step, and resolution holds.
    assert!(
        battle.pending_numbers.is_empty(),
        "no number pops on the same tick as the animation"
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
    // Draining the deferred step (what `resolve_tick` does once the animation has
    // played out) is what finally pops the number.
    battle.resolve_next();
    assert!(
        !battle.pending_numbers.is_empty(),
        "the number pops only once the deferred impact resolves"
    );
}

#[test]
fn a_zero_animation_strike_applies_immediately_without_holding() {
    // build_1v2's hero is bare-handed (unarmed_animation 0), so there is no swing
    // to wait for: the impact must land at once with no hold (never wedging).
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // land the blow deterministically
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
        !battle.pending_numbers.is_empty(),
        "its damage number pops immediately"
    );
}

#[test]
fn an_enemy_normal_attack_applies_and_pops_a_number_without_holding() {
    // An rpg2k enemy normal attack plays no animation, so it applies immediately
    // (paced only by the step timer), queues no animation, and never holds.
    let mut battle = build_1v2();
    wind_enemy_hits(&mut battle, &[0]); // land the enemy to-hit roll
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
        !battle.pending_numbers.is_empty(),
        "the damage number pops on this tick"
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
    // Once it appears it plays for its duration.
    assert!(battle.tick_anim_hold(true), "holds while it plays");
    assert!(battle.tick_anim_hold(true));
    // When the seen animation is gone, the hold releases.
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
