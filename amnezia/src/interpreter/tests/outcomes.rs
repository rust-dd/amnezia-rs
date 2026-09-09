use super::*;
use crate::battle::BattleOutcome;

fn encounter(troop: i32, escape_mode: i32, defeat_mode: i32) -> EventCommand {
    let mut command = cmd(10710, 0, vec![0, troop, 1, escape_mode, defeat_mode, 0]);
    command.string = "Town".into();
    command
}

#[test]
fn consecutive_unbranched_encounters_both_start_and_preserve_their_options() {
    let mut app = interp_app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![encounter(5, 0, 0), encounter(6, 2, 0), switch_cmd(90, 0, 0)],
    );
    app.update();
    let requests = app
        .world_mut()
        .resource_mut::<Messages<BattleRequest>>()
        .drain()
        .collect::<Vec<_>>();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].troop_id, 5);
    assert_eq!(requests[0].background, "Town");
    assert!(!requests[0].allow_escape);
    app.world_mut().resource_mut::<BattleResult>().0 = Some(BattleOutcome::Victory);
    app.update();
    let requests = app
        .world_mut()
        .resource_mut::<Messages<BattleRequest>>()
        .drain()
        .collect::<Vec<_>>();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].troop_id, 6);
    assert!(requests[0].allow_escape);
    assert!(!switch_on(&app, 90));
    app.world_mut().resource_mut::<BattleResult>().0 = Some(BattleOutcome::Victory);
    app.update();
    assert!(switch_on(&app, 90));
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn an_unhandled_defeat_cannot_use_a_later_encounters_defeat_branch() {
    let mut app = interp_app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            encounter(5, 0, 0),
            encounter(6, 0, 1),
            cmd(20712, 0, vec![]),
            switch_cmd(90, 0, 1),
            cmd(20713, 0, vec![]),
        ],
    );
    app.update();
    app.world_mut().resource_mut::<BattleResult>().0 = Some(BattleOutcome::Defeat);
    app.update();
    assert!(app.world().resource::<GameOverActive>().0);
    assert!(!switch_on(&app, 90));
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn custom_battle_outcome_resumes_the_correct_branch_and_then_the_event() {
    for (outcome, selected) in [
        (BattleOutcome::Victory, 1),
        (BattleOutcome::Escape, 2),
        (BattleOutcome::Defeat, 3),
        (BattleOutcome::Abort, 0),
    ] {
        let mut app = interp_app();
        app.world_mut().resource_mut::<RunningEvent>().start(
            1,
            vec![
                encounter(5, 2, 1),
                cmd(20710, 0, vec![]),
                switch_cmd(1, 0, 1),
                cmd(20711, 0, vec![]),
                switch_cmd(2, 0, 1),
                cmd(20712, 0, vec![]),
                switch_cmd(3, 0, 1),
                cmd(20713, 0, vec![]),
                switch_cmd(90, 0, 0),
            ],
        );
        app.update();
        app.world_mut().resource_mut::<BattleResult>().0 = Some(outcome);
        app.update();
        for id in 1..=3 {
            assert_eq!(switch_on(&app, id), id == selected);
        }
        assert!(switch_on(&app, 90));
        assert!(!app.world().resource::<GameOverActive>().0);
    }
}

#[test]
fn shop_and_inn_handlers_continue_after_their_own_end_marker() {
    for (open, accepted, cancelled, end) in
        [(10720, 20720, 20721, 20722), (10730, 20730, 20731, 20732)]
    {
        for transacted in [false, true] {
            let mut app = interp_app();
            app.world_mut().resource_mut::<RunningEvent>().start(
                1,
                vec![
                    cmd(open, 0, vec![0, 10, 0, 0]),
                    cmd(accepted, 0, vec![]),
                    switch_cmd(1, 0, 1),
                    cmd(cancelled, 0, vec![]),
                    switch_cmd(2, 0, 1),
                    cmd(end, 0, vec![]),
                    switch_cmd(90, 0, 0),
                ],
            );
            app.update();
            app.world_mut().resource_mut::<ShopOutcome>().transacted = transacted;
            app.update();
            assert_eq!(switch_on(&app, 1), transacted);
            assert_eq!(switch_on(&app, 2), !transacted);
            assert!(switch_on(&app, 90));
            assert!(!app.world().resource::<RunningEvent>().active());
        }
    }
}
