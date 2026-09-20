use super::*;
use crate::battle::BattleOutcome;

mod originals;
mod stack;

fn option(index: i32, label: &str) -> EventCommand {
    let mut command = cmd(20140, 0, vec![index]);
    command.string = label.into();
    command
}

fn select(app: &mut App, index: i32) {
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    assert!(app.world().resource::<Choice>().active());
    let mut choice = app.world_mut().resource_mut::<Choice>();
    choice.active = false;
    choice.result = Some(index);
    crate::dialogue::testing::update_prompt(app.world_mut());
    app.update();
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
}

#[test]
fn a_called_choice_opens_independently_and_preserves_the_callers_selection() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![map_event(
            2,
            0,
            vec![
                cmd(10140, 0, vec![0]),
                option(0, "A"),
                switch_cmd(901, 0, 1),
                option(1, "B"),
                switch_cmd(902, 0, 1),
                cmd(20141, 0, vec![]),
            ],
        )],
    });
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            cmd(10140, 0, vec![0]),
            option(0, "Szülő A"),
            cmd(12330, 1, vec![1, 2, 1]),
            switch_cmd(903, 0, 1),
            option(1, "Szülő B"),
            switch_cmd(904, 0, 1),
            cmd(20141, 0, vec![]),
            switch_cmd(905, 0, 0),
        ],
    );
    app.update();
    select(&mut app, 0);
    assert!(app.world().resource::<Choice>().active());
    assert_eq!(app.world().resource::<Choice>().options, ["A", "B"]);
    assert!(!switch_on(&app, 901));
    assert!(!switch_on(&app, 902));
    select(&mut app, 1);
    for id in [902, 903, 905] {
        assert!(switch_on(&app, id));
    }
    for id in [901, 904] {
        assert!(!switch_on(&app, id));
    }
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn a_nested_battle_restores_the_callers_result_after_the_child_finishes() {
    let mut app = interp_app();
    let encounter = || cmd(10710, 0, vec![0, 5, 1, 2, 1, 0]);
    app.insert_resource(MapEvents {
        events: vec![map_event(
            2,
            0,
            vec![
                encounter(),
                cmd(20712, 0, vec![]),
                switch_cmd(902, 0, 1),
                cmd(20713, 0, vec![]),
            ],
        )],
    });
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            encounter(),
            cmd(20710, 0, vec![]),
            cmd(12330, 1, vec![1, 2, 1]),
            cmd(11410, 1, vec![0]),
            switch_cmd(903, 0, 1),
            cmd(20713, 0, vec![]),
        ],
    );
    app.update();
    app.world_mut().resource_mut::<BattleResult>().0 = Some(BattleOutcome::Victory);
    app.update();
    assert!(app.world().resource::<RunningEvent>().frame.battle_pending);
    app.world_mut().resource_mut::<BattleResult>().0 = Some(BattleOutcome::Defeat);
    app.update();
    let frame = &app.world().resource::<RunningEvent>().frame;
    assert_eq!(frame.event_id, 1);
    assert_eq!(frame.battle_outcome, Some(BattleOutcome::Victory));
    assert!(switch_on(&app, 902));
    app.update();
    assert!(switch_on(&app, 903));
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn a_called_merchant_restores_the_callers_transaction_result() {
    for (open, accepted, cancelled, end) in
        [(10720, 20720, 20721, 20722), (10730, 20730, 20731, 20732)]
    {
        let mut app = interp_app();
        app.insert_resource(MapEvents {
            events: vec![map_event(
                2,
                0,
                vec![
                    cmd(open, 0, vec![0, 10, 0, 0]),
                    cmd(cancelled, 0, vec![]),
                    switch_cmd(902, 0, 1),
                    cmd(end, 0, vec![]),
                ],
            )],
        });
        app.world_mut().resource_mut::<RunningEvent>().start(
            1,
            vec![
                cmd(open, 0, vec![0, 10, 0, 0]),
                cmd(accepted, 0, vec![]),
                cmd(12330, 1, vec![1, 2, 1]),
                cmd(11410, 1, vec![0]),
                switch_cmd(903, 0, 1),
                cmd(end, 0, vec![]),
            ],
        );
        app.update();
        app.world_mut().resource_mut::<ShopOutcome>().transacted = true;
        app.update();
        assert!(app.world().resource::<RunningEvent>().frame.shop_pending);
        app.world_mut().resource_mut::<ShopOutcome>().transacted = false;
        app.update();
        let frame = &app.world().resource::<RunningEvent>().frame;
        assert_eq!(frame.event_id, 1);
        assert_eq!(frame.shop_transacted, Some(true));
        assert!(switch_on(&app, 902));
        app.update();
        assert!(switch_on(&app, 903));
        assert!(!app.world().resource::<RunningEvent>().active());
    }
}
