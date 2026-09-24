use super::*;
use crate::battle::BattleOutcome;

#[test]
fn saved_wait_and_key_input_continue_identically_to_an_uninterrupted_event() {
    for fps in [15, 30, 60, 144] {
        let (mut app, path) = app(&format!("continuation-{fps}"));
        let mut control = interp_app();
        for app in [&mut app, &mut control] {
            app.add_plugins((
                crate::timing::TimingPlugin,
                crate::timing::logical::LogicalPlugin,
            ));
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / 60.0),
            ));
            app.update();
        }
        app.world_mut().resource_mut::<RunningEvent>().start(
            0,
            vec![
                switch_cmd(901, 2, 0),
                cmd(11410, 0, vec![3]),
                cmd(11610, 0, vec![9, 1, 1, 1, 1]),
                switch_cmd(902, 2, 0),
            ],
        );
        app.update();
        let before = app.world().resource::<RunningEvent>().frame.clone();
        control.world_mut().resource_mut::<RunningEvent>().frame = before.clone();
        control
            .world_mut()
            .resource_mut::<Switches>()
            .set(901, true);
        app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
        app.update();
        let bytes = std::fs::read(&path).unwrap();
        load(&mut app);
        for _ in 0..8 {
            app.update();
            assert_eq!(app.world().resource::<RunningEvent>().frame, before);
        }
        app.world_mut()
            .resource_mut::<crate::transitions::Transition>()
            .clear();
        for app in [&mut app, &mut control] {
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / fps as f64),
            ));
        }
        for tick in 0..fps * 2 {
            for app in [&mut app, &mut control] {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                *keys = default();
                if tick == fps {
                    keys.press(KeyCode::ArrowRight);
                }
                app.update();
            }
            assert_eq!(
                app.world().resource::<RunningEvent>().frame,
                control.world().resource::<RunningEvent>().frame,
                "{fps} FPS, tick {tick}"
            );
            assert_eq!(
                app.world().resource::<Variables>().get(9),
                control.world().resource::<Variables>().get(9)
            );
            assert_eq!(switch_on(&app, 902), switch_on(&control, 902));
            assert!(switch_on(&app, 901));
        }
        assert!(switch_on(&app, 902));
        assert_eq!(app.world().resource::<Variables>().get(9), 3);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn a_save_inside_a_chosen_option_keeps_the_selection_until_its_end_marker() {
    let (mut app, path) = app("choice");
    let mut first = cmd(20140, 0, vec![0]);
    first.string = "Első".into();
    let mut second = cmd(20140, 0, vec![1]);
    second.string = "Második".into();
    app.world_mut().resource_mut::<RunningEvent>().start(
        0,
        vec![
            cmd(10140, 0, vec![0]),
            first,
            switch_cmd(901, 2, 1),
            cmd(11910, 1, vec![]),
            switch_cmd(902, 2, 1),
            second,
            switch_cmd(903, 2, 1),
            cmd(20141, 0, vec![]),
            switch_cmd(904, 2, 0),
        ],
    );
    app.update();
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    assert!(app.world().resource::<Choice>().active());
    app.world_mut().resource_mut::<Choice>().active = false;
    app.world_mut().resource_mut::<Choice>().result = Some(0);
    crate::dialogue::testing::update_prompt(app.world_mut());
    app.update();
    crate::dialogue::testing::finish_window_close(app.world_mut());
    app.update();
    let before = app.world().resource::<RunningEvent>().frame.clone();
    assert_eq!(before.choices.get(&0), Some(&0));
    app.update();
    load(&mut app);
    assert_eq!(app.world().resource::<RunningEvent>().frame, before);
    resume(&mut app);
    for id in [901, 902, 904] {
        assert!(switch_on(&app, id));
    }
    assert!(!switch_on(&app, 903));
    assert!(!app.world().resource::<Choice>().active());
    assert!(!app.world().resource::<RunningEvent>().active());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn battle_and_merchant_results_survive_a_save_before_their_handler() {
    for (index, (battle, merchant, selected)) in [
        (Some(BattleOutcome::Victory), None, 901),
        (Some(BattleOutcome::Escape), None, 902),
        (Some(BattleOutcome::Defeat), None, 903),
        (Some(BattleOutcome::Abort), None, 0),
        (None, Some(true), 901),
        (None, Some(false), 902),
    ]
    .into_iter()
    .enumerate()
    {
        let (mut app, path) = app(&format!("outcome-{index}"));
        let mut commands = vec![cmd(11910, 0, vec![])];
        let handlers = if merchant.is_some() {
            vec![20720, 20721]
        } else {
            vec![20710, 20711, 20712]
        };
        for (branch, handler) in handlers.into_iter().enumerate() {
            commands.push(cmd(handler, 0, vec![]));
            commands.push(switch_cmd(901 + branch as i32, 2, 1));
        }
        commands.push(cmd(
            if merchant.is_some() { 20722 } else { 20713 },
            0,
            vec![],
        ));
        commands.push(switch_cmd(904, 2, 0));
        let mut running = app.world_mut().resource_mut::<RunningEvent>();
        running.start(0, commands);
        running.frame.battle_outcome = battle;
        running.frame.shop_transacted = merchant;
        app.update();
        let before = app.world().resource::<RunningEvent>().frame.clone();
        app.update();
        load(&mut app);
        assert_eq!(app.world().resource::<RunningEvent>().frame, before);
        resume(&mut app);
        for id in 901..=903 {
            assert_eq!(switch_on(&app, id), id == selected);
        }
        assert!(switch_on(&app, 904));
        assert!(!app.world().resource::<RunningEvent>().active());
        std::fs::remove_file(path).unwrap();
    }
}
