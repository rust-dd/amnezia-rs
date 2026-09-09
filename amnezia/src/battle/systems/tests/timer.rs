use super::*;

#[test]
fn expired_mission_timer_returns_to_the_map_without_input_or_rewards() {
    let mut app = logic_app();
    app.add_systems(Update, abort_expired_battle.before(outcome_input));
    app.init_resource::<crate::timer::GameClock>();
    app.insert_resource(CurrentBgm::with_track("Mission", 0.8, 1.0));
    app.world_mut().write_message(BattleRequest {
        troop_id: DEBUG_TROOP,
        ..default()
    });
    app.update();
    assert!(app.world().resource::<BattleActive>().0);
    app.world_mut().resource_mut::<Battle>().members[0].hp = 17;
    app.world_mut()
        .resource_mut::<crate::timer::GameClock>()
        .expired = true;
    app.update();
    assert!(!app.world().resource::<BattleActive>().0);
    assert_eq!(
        app.world().resource::<BattleResult>().0,
        Some(BattleOutcome::Abort)
    );
    assert_eq!(app.world().resource::<Inventory>().gold(), 0);
    assert!(app.world().resource::<Progression>().entries().is_empty());
    assert_eq!(
        app.world().resource::<Vitals>().entries(),
        vec![(1, (17, 37))]
    );
    assert!(app.world().resource::<Battle>().phase == Phase::Inactive);
    let audio = app
        .world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect::<Vec<_>>();
    assert!(matches!(audio.last(), Some(AudioRequest::Bgm { name, .. }) if name == "Mission"));
}
