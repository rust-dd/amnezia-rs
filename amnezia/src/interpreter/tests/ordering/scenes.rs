use super::*;

mod lifecycle;

pub(super) fn scene(code: u32) -> EventCommand {
    match code {
        10710 => cmd(code, 0, vec![0, 5, 1, 2, 1, 0]),
        10720 => cmd(code, 0, vec![0, 0, 0, 1, 3]),
        _ => cmd(code, 0, vec![]),
    }
}

pub(super) fn battle_requests(app: &mut App) -> Vec<BattleRequest> {
    app.world_mut()
        .resource_mut::<Messages<BattleRequest>>()
        .drain()
        .collect()
}

pub(super) fn shop_requests(app: &mut App) -> Vec<ShopRequest> {
    app.world_mut()
        .resource_mut::<Messages<ShopRequest>>()
        .drain()
        .collect()
}

#[test]
fn a_parallel_save_request_finishes_character_updates_and_limits_later_interpreters() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![scene(11910)]),
            map_event(2, 4, vec![switch_cmd(10, 0, 0), switch_cmd(11, 0, 0)]),
        ],
    });
    let world = app.world_mut();
    *world
        .query::<&mut RouteStepper>()
        .single_mut(world)
        .unwrap() = RouteStepper::from_move_event(&[10001, 8, 0, 0, 1]);
    world
        .resource_mut::<RunningEvent>()
        .start(0, vec![switch_cmd(20, 0, 0), switch_cmd(21, 0, 0)]);
    app.update();
    assert!(switch_on(&app, 10));
    assert!(!switch_on(&app, 11));
    assert!(switch_on(&app, 20));
    assert!(!switch_on(&app, 21));
    assert_eq!(hero_x(&mut app), 6);
    assert!(app.world().resource::<EventSaveRequest>().0);
}

#[test]
fn the_last_parallel_scene_request_replaces_every_earlier_scene_kind() {
    for first in [10710, 10720, 11910, 12420] {
        for last in [10710, 10720, 11910, 12420] {
            let mut app = app();
            app.insert_resource(MapEvents {
                events: vec![
                    map_event(2, 4, vec![scene(last)]),
                    map_event(1, 4, vec![scene(first)]),
                ],
            });
            app.update();
            assert_eq!(
                battle_requests(&mut app).len(),
                usize::from(last == 10710),
                "{first} -> {last}"
            );
            assert_eq!(
                shop_requests(&mut app).len(),
                usize::from(last == 10720),
                "{first} -> {last}"
            );
            assert_eq!(
                app.world().resource::<EventSaveRequest>().0,
                last == 11910,
                "{first} -> {last}"
            );
            assert_eq!(
                app.world().resource::<GameOverActive>().0,
                last == 12420,
                "{first} -> {last}"
            );
        }
    }
}

#[test]
fn common_then_map_then_foreground_requests_share_one_replacement_slot() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![common(1, 4, 0, vec![scene(10710)])]));
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 4, vec![scene(10720)])],
    });
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, vec![scene(11910)]);
    app.update();
    assert!(battle_requests(&mut app).is_empty());
    assert!(shop_requests(&mut app).is_empty());
    assert!(app.world().resource::<EventSaveRequest>().0);
}

#[test]
fn superseded_battles_do_not_wait_for_or_consume_another_events_result() {
    let mut app = app();
    let mut first = scene(10710);
    first.params[1] = 6;
    app.insert_resource(MapEvents {
        events: vec![
            map_event(
                1,
                4,
                vec![
                    first,
                    cmd(20710, 0, vec![]),
                    switch_cmd(10, 0, 1),
                    cmd(20713, 0, vec![]),
                    switch_cmd(11, 0, 0),
                    cmd(11410, 0, vec![100]),
                ],
            ),
            map_event(
                2,
                4,
                vec![
                    scene(10710),
                    cmd(20710, 0, vec![]),
                    switch_cmd(20, 0, 1),
                    cmd(20713, 0, vec![]),
                    switch_cmd(21, 0, 0),
                    cmd(11410, 0, vec![100]),
                ],
            ),
        ],
    });
    app.update();
    let requests = battle_requests(&mut app);
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].troop_id, 5);
    app.world_mut().resource_mut::<BattleResult>().0 = Some(crate::battle::BattleOutcome::Victory);
    app.update();
    assert!(!switch_on(&app, 10));
    assert!(switch_on(&app, 11));
    assert!(switch_on(&app, 20));
    assert!(switch_on(&app, 21));
}

#[test]
fn a_superseded_shop_skips_both_transaction_branches() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(
                1,
                4,
                vec![
                    scene(10720),
                    cmd(20720, 0, vec![]),
                    switch_cmd(10, 0, 1),
                    cmd(20721, 0, vec![]),
                    switch_cmd(11, 0, 1),
                    cmd(20722, 0, vec![]),
                    switch_cmd(12, 0, 0),
                    cmd(11410, 0, vec![100]),
                ],
            ),
            map_event(2, 4, vec![scene(11910), cmd(11410, 0, vec![100])]),
        ],
    });
    app.update();
    assert!(shop_requests(&mut app).is_empty());
    app.world_mut().resource_mut::<EventSaveRequest>().0 = false;
    app.update();
    assert!(!switch_on(&app, 10));
    assert!(!switch_on(&app, 11));
    assert!(switch_on(&app, 12));
}

#[test]
fn game_over_requests_yield_without_destroying_the_interpreter() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, vec![scene(12420), switch_cmd(10, 0, 0)]);
    app.update();
    let running = app.world().resource::<RunningEvent>();
    assert!(running.active());
    assert_eq!(running.frame.ip, 1);
    assert!(app.world().resource::<GameOverActive>().0);
    assert!(!switch_on(&app, 10));
}

#[test]
fn returning_to_title_discards_an_earlier_scene_request() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![scene(10710)]),
            map_event(2, 4, vec![scene(12510)]),
        ],
    });
    app.update();
    assert!(app.world().resource::<TitleActive>().0);
    assert!(battle_requests(&mut app).is_empty());
}
