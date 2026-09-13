use super::*;
use crate::save::{EventSaveRequest, LoadOutcome, LoadRequest, SaveLocation, SavePlugin};

mod call_scope;
mod continuation;
mod erasure;
mod fixture;
mod originals;
mod validation;
use fixture::{app, load, resume};

#[test]
fn a_saved_crystal_resumes_after_the_save_without_replaying_or_losing_its_tail() {
    let (mut app, path) = app("crystal");
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0002.ron",
        crate::assets::asset_root()
    ));
    let page = map
        .events
        .iter()
        .flat_map(|event| &event.pages)
        .find(|page| page.commands.iter().any(|command| command.code == 11910))
        .unwrap();
    let mut commands = vec![switch_cmd(901, 2, 0)];
    commands.extend(page.commands.clone());
    commands.push(switch_cmd(902, 2, 0));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, commands);
    app.update();
    let saved_ip = app.world().resource::<RunningEvent>().frame.ip;
    app.update();
    assert!(switch_on(&app, 902));
    let bytes = std::fs::read(&path).unwrap();
    load(&mut app);
    assert!(app.world().resource::<RunningEvent>().active());
    assert_eq!(app.world().resource::<RunningEvent>().frame.ip, saved_ip);
    assert!(switch_on(&app, 901));
    assert!(!switch_on(&app, 902));
    assert!(!app.world().resource::<EventSaveRequest>().0);
    resume(&mut app);
    for _ in 0..4 {
        app.update();
        assert!(switch_on(&app, 901));
        assert!(switch_on(&app, 902));
        assert!(!app.world().resource::<RunningEvent>().active());
        assert!(!app.world().resource::<EventSaveRequest>().0);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_loaded_nested_save_restores_both_callers_before_a_new_autorun_can_start() {
    let (mut app, path) = app("nested");
    let mut autorun = map_event(3, 3, vec![switch_cmd(905, 0, 0)]);
    autorun.pages[0].condition.flags = 1;
    autorun.pages[0].condition.switch_a = 901;
    app.insert_resource(MapEvents {
        events: vec![
            map_event(
                2,
                0,
                vec![
                    switch_cmd(902, 2, 0),
                    cmd(11910, 0, vec![]),
                    switch_cmd(903, 2, 0),
                ],
            ),
            autorun,
        ],
    });
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            switch_cmd(901, 0, 0),
            cmd(12330, 0, vec![1, 2, 1]),
            switch_cmd(904, 2, 0),
            switch_cmd(901, 1, 0),
        ],
    );
    app.update();
    app.update();
    assert!(switch_on(&app, 904));
    let bytes = std::fs::read(&path).unwrap();
    load(&mut app);
    let frame = &app.world().resource::<RunningEvent>().frame;
    assert!(frame.active());
    assert_eq!(frame.event_id, 2);
    assert_eq!(frame.ip, 2);
    assert_eq!(frame.call_stack.len(), 1);
    assert_eq!(frame.call_stack[0].event_id, 1);
    assert_eq!(frame.call_stack[0].ip, 2);
    resume(&mut app);
    for _ in 0..4 {
        app.update();
        for id in [902, 903, 904] {
            assert!(switch_on(&app, id));
        }
        assert!(!switch_on(&app, 901));
        assert!(!switch_on(&app, 905));
        assert!(!app.world().resource::<RunningEvent>().active());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_loaded_foreground_keeps_its_time_movement_and_key_waits() {
    for (tag, command) in [
        ("time", cmd(11410, 0, vec![10])),
        ("movement", cmd(11340, 0, vec![])),
        ("key", cmd(11610, 0, vec![9, 1, 1, 1, 1])),
    ] {
        let (mut app, path) = app(tag);
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(0, vec![command, switch_cmd(901, 2, 0)]);
        app.update();
        let frame = &app.world().resource::<RunningEvent>().frame;
        let before = (
            frame.ip,
            frame.wait,
            frame.wait_movement,
            frame.key_pending,
            frame.key_var,
            frame.key_accept,
        );
        app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
        app.update();
        load(&mut app);
        let frame = &app.world().resource::<RunningEvent>().frame;
        assert!(frame.active(), "{tag}");
        assert_eq!(
            (
                frame.ip,
                frame.wait,
                frame.wait_movement,
                frame.key_pending,
                frame.key_var,
                frame.key_accept
            ),
            before,
            "{tag}"
        );
        assert!(!switch_on(&app, 901));
        std::fs::remove_file(path).unwrap();
    }
}
