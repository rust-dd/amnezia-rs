use super::*;
use crate::teleport::{Fade, PendingTeleport, TeleportPlugin};
use crate::timing::{SceneFrames, TimingPlugin, logical::LogicalPlugin};
use crate::world::MapData;
use bevy::time::TimeUpdateStrategy;

#[test]
fn a_destination_inn_keeps_its_preupdate_suspended_while_playing_music() {
    let mut app = interpreter_app();
    app.add_plugins((
        AssetPlugin::default(),
        TeleportPlugin,
        TimingPlugin,
        LogicalPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<AudioSource>()
    .init_resource::<crate::audio::MemorizedBgm>()
    .add_message::<crate::world::MapChanged>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
    .add_systems(
        Update,
        crate::audio::flush.after(crate::interpreter::InterpreterStep),
    );
    let world = app.world_mut();
    let hero = world
        .query_filtered::<Entity, With<crate::player::Player>>()
        .single(world)
        .unwrap();
    world.entity_mut(hero).insert(Transform::default());
    world.resource_mut::<MapData>().map_id = 3;
    world.resource_mut::<SystemMusic>().inn = amnezia_data::MusicDef {
        name: "Inn".into(),
        volume: 100,
        tempo: 100,
        ..default()
    };
    app.update();
    let mut inn = command(10730, vec![0, 0, 1]);
    inn.indent = 1;
    let mut completed = increment(1);
    completed.indent = 1;
    let mut wait = command(11410, vec![1000]);
    wait.indent = 1;
    app.insert_resource(CommonEvents(vec![common(
        1,
        vec![
            command(10220, vec![0, 10, 10, 0, 6, 10001, 1]),
            command(12010, vec![1, 10, 0, 7, 0]),
            inn,
            completed,
            wait,
            command(22011, vec![]),
        ],
    )]));
    app.world_mut()
        .resource_mut::<PendingTeleport>()
        .reserve((3, 7, 8), false);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    for _ in 0..100 {
        app.update();
        if matches!(app.world().resource::<State>().phase, Phase::Resting { .. }) {
            break;
        }
    }
    assert!(matches!(
        app.world().resource::<State>().phase,
        Phase::Resting { .. }
    ));
    assert!(app.world().resource::<Fade>().busy());
    assert!(!app.world().resource::<Transition>().busy());
    let scene = app.world().resource::<SceneFrames>().frame;
    for offset in 1..=10 {
        app.update();
        assert_eq!(app.world().resource::<SceneFrames>().frame, scene + offset);
        assert_eq!(counts(&app)[0], 0);
        assert!(app.world().resource::<Fade>().busy());
        assert!(!app.world().resource::<Transition>().busy());
        assert!(matches!(
            app.world().resource::<State>().phase,
            Phase::Resting { .. }
        ));
    }
    app.world_mut().write_message(AudioRequest::StopBgm);
    crate::audio::flush(app.world_mut());
    for _ in 0..100 {
        app.update();
        if !app.world().resource::<Fade>().busy() {
            break;
        }
    }
    assert!(!app.world().resource::<State>().active());
    assert!(!app.world().resource::<Fade>().busy());
    assert_eq!(counts(&app)[0], 1);
}
