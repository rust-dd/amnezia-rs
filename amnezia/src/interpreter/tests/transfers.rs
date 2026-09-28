use super::*;

mod reservations;

#[test]
fn airship_parallel_transfer_restores_music_and_tint_before_the_map_unloads() {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0094.ron",
        crate::assets::asset_root()
    ));
    let commands = &map.events.iter().find(|e| e.name == "Ron").unwrap().pages[0].commands;
    let transfer = commands.iter().position(|c| c.code == 10810).unwrap();
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![map_event(2, 4, commands[transfer..].to_vec())],
    });
    app.update();
    assert_eq!(
        app.world().resource::<PendingTeleport>().0,
        Some((86, 10, 7))
    );
    let audio = app
        .world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect::<Vec<_>>();
    assert!(
        audio
            .iter()
            .any(|message| matches!(message, AudioRequest::Bgm { name, .. } if name == "Memories"))
    );
    let effects = app
        .world_mut()
        .resource_mut::<Messages<ScreenEffect>>()
        .drain()
        .collect::<Vec<_>>();
    assert!(effects.contains(&ScreenEffect::Tint {
        r: 100,
        g: 100,
        b: 100,
        sat: 100,
        secs: 1.0
    }));
}

#[test]
fn foreground_transfer_waits_for_arrival_before_running_the_next_command() {
    let mut app = interp_app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        2,
        vec![cmd(10810, 0, vec![86, 10, 7]), switch_cmd(77, 0, 0)],
    );
    app.update();
    assert!(!switch_on(&app, 77));
    app.world_mut().resource_mut::<PendingTeleport>().0 = None;
    app.update();
    assert!(switch_on(&app, 77));
}
