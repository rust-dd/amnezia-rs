use super::*;
use crate::timing::GameFrames;
use crate::transitions::{Defaults, Settings, TransitionPlugin};

mod accepted;
mod inns;

fn app() -> App {
    let mut app = interp_app();
    app.add_plugins((
        AssetPlugin::default(),
        TransitionPlugin,
        crate::teleport::TeleportPlugin,
    ))
    .init_asset::<Image>()
    .add_message::<crate::world::MapChanged>();
    let mut data = MapData::for_test(20, 15);
    data.map_id = 3;
    app.insert_resource(data);
    let hero = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(hero)
        .insert(Transform::default());
    let mut settings = app.world_mut().resource_mut::<Settings>();
    settings.change(&[0, 20], &Defaults([0; 6]));
    settings.change(&[1, 19], &Defaults([0; 6]));
    app.update();
    app
}

fn arrive(app: &mut App, map: u32, reload: bool) {
    if reload {
        app.world_mut()
            .resource_mut::<PendingTeleport>()
            .reload(map, 7, 6);
    } else {
        app.world_mut().resource_mut::<PendingTeleport>().0 = Some((map, 7, 6));
    }
    app.update();
    app.world_mut().resource_mut::<GameFrames>().frame += 35;
    app.update();
    assert_eq!(app.world().resource::<MapData>().map_id, map);
    assert!(app.world().resource::<Fade>().busy());
}

fn finish_transfer(app: &mut App) {
    app.update();
    app.world_mut().resource_mut::<GameFrames>().frame += 35;
    app.update();
    assert!(!app.world().resource::<Fade>().busy());
}

fn message() -> EventCommand {
    EventCommand {
        string: "Transfer message".into(),
        ..cmd(10110, 0, vec![])
    }
}

fn parallel(app: &mut App, commands: Vec<EventCommand>) {
    set_switch(app, 300, true);
    app.insert_resource(CommonEvents(vec![common(1, 4, 300, commands)]));
    super::super::parallel::run_parallel(app.world_mut());
    assert!(app.world().resource::<Dialogue>().active);
}

#[test]
fn ordinary_cross_map_transfer_clears_only_the_foreground_stack_event_ids() {
    let mut app = app();
    {
        let mut running = app.world_mut().resource_mut::<RunningEvent>();
        running.start(7, vec![switch_cmd(80, 0, 0)]);
        running.frame.decision = true;
        running.frame.choices.insert(2, 1);
        assert!(running.frame.call(vec![cmd(11410, 0, vec![10])], 9));
        running.frame.wait = 5.0;
        running.frame.shop_transacted = Some(true);
    }
    let mut expected = app.world().resource::<RunningEvent>().frame.clone();
    expected.event_id = 0;
    expected.call_stack[0].event_id = 0;
    arrive(&mut app, 4, false);
    assert_eq!(app.world().resource::<RunningEvent>().frame, expected);
}

#[test]
fn same_map_transfers_and_save_reloads_preserve_event_ids_and_parallel_messages() {
    for (map, reload) in [(3, false), (3, true), (4, true)] {
        let mut app = app();
        parallel(&mut app, vec![message()]);
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(7, vec![cmd(11410, 0, vec![10])]);
        arrive(&mut app, map, reload);
        assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(7));
        assert!(app.world().resource::<Dialogue>().active);
    }
}

#[test]
fn cross_map_transfer_closes_parallel_text_but_keeps_foreground_text() {
    for foreground in [false, true] {
        let mut app = app();
        if foreground {
            app.world_mut()
                .resource_mut::<RunningEvent>()
                .start(7, vec![message()]);
            super::super::driver::foreground(app.world_mut());
        } else {
            parallel(&mut app, vec![message()]);
        }
        arrive(&mut app, 4, false);
        let dialogue = app.world().resource::<Dialogue>();
        assert_eq!(dialogue.active, foreground);
        assert_eq!(dialogue.lifecycle.message.closing(), !foreground);
        assert!(dialogue.busy());
    }
}

fn choices() -> Vec<EventCommand> {
    let mut commands = vec![cmd(10140, 0, vec![5])];
    for index in [0, 1, 4] {
        commands.push(EventCommand {
            string: format!("Option {index}"),
            ..cmd(20140, 0, vec![index])
        });
        commands.push(switch_cmd(40 + index, 0, 1));
    }
    commands.push(cmd(20141, 0, vec![]));
    commands
}

#[test]
fn abandoned_parallel_prompts_do_not_select_a_branch_write_a_number_or_reopen() {
    for numeric in [false, true] {
        for embedded in [false, true] {
            let mut app = app();
            app.world_mut().resource_mut::<Variables>().set(50, 77);
            let mut commands = if embedded { vec![message()] } else { vec![] };
            commands.extend(if numeric {
                vec![cmd(10150, 0, vec![3, 50])]
            } else {
                choices()
            });
            commands.extend([
                cmd(10220, 0, vec![0, 90, 90, 1, 0, 1]),
                switch_cmd(300, 1, 0),
            ]);
            parallel(&mut app, commands);
            crate::dialogue::testing::finish_prompt_text(app.world_mut());
            if numeric {
                assert!(app.world().resource::<InputNumber>().active());
                app.world_mut().resource_mut::<InputNumber>().value = 999;
            } else {
                assert!(app.world().resource::<Choice>().active());
            }
            arrive(&mut app, 4, false);
            assert!(!app.world().resource::<Dialogue>().active);
            assert!(!app.world().resource::<Choice>().active());
            assert!(!app.world().resource::<InputNumber>().active());
            assert!(app.world().resource::<Choice>().result.is_none());
            assert!(app.world().resource::<InputNumber>().result.is_none());
            finish_transfer(&mut app);
            crate::dialogue::testing::finish_window_close(app.world_mut());
            for _ in 0..3 {
                app.update();
            }
            assert!(!app.world().resource::<Dialogue>().active);
            assert_eq!(app.world().resource::<Variables>().get(50), 77);
            assert_eq!(app.world().resource::<Variables>().get(90), 1);
            for id in [40, 41, 44] {
                assert!(!switch_on(&app, id));
            }
        }
    }
}

#[test]
fn ordinary_transfer_preserves_foreground_embedded_prompts() {
    for numeric in [false, true] {
        let mut app = app();
        let mut commands = vec![message()];
        commands.extend(if numeric {
            vec![cmd(10150, 0, vec![3, 50])]
        } else {
            choices()
        });
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(7, commands);
        super::super::driver::foreground(app.world_mut());
        crate::dialogue::testing::finish_prompt_text(app.world_mut());
        arrive(&mut app, 4, false);
        assert!(app.world().resource::<Dialogue>().active);
        assert_eq!(app.world().resource::<InputNumber>().active(), numeric);
        assert_eq!(app.world().resource::<Choice>().active(), !numeric);
    }
}

#[test]
fn a_quick_transfer_keeps_the_parallel_message_and_number_input() {
    let mut app = app();
    parallel(&mut app, vec![cmd(10150, 0, vec![3, 50])]);
    app.world_mut()
        .resource_mut::<PendingTeleport>()
        .quick(4, 7, 6);
    crate::teleport::flush_quick(app.world_mut());
    assert!(app.world().resource::<Dialogue>().active);
    assert!(app.world().resource::<InputNumber>().active());
    assert!(!app.world().resource::<Fade>().busy());
}

#[test]
fn a_removed_map_page_does_not_leave_an_active_prompt_after_ordinary_transfer() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![map_event(8, 4, vec![cmd(10150, 0, vec![3, 50])])],
    });
    super::super::parallel::run_parallel(app.world_mut());
    assert!(app.world().resource::<InputNumber>().active());
    arrive(&mut app, 4, false);
    assert!(!app.world().resource::<Dialogue>().active);
    assert!(!app.world().resource::<InputNumber>().active());
    assert_eq!(app.world().resource::<ParallelPool>().count(), 0);
}
