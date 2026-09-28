use super::*;
use crate::audio::{AudioRequest, BgmTrack, CurrentBgm, MemorizedBgm, saved::MusicState};
use crate::player::{CameraPan, Player};
use crate::world::{MapChanged, MapData, MapRebuilt, MoveQueue, RouteStepper};

#[test]
fn inline_restoration_exposes_hero_camera_and_music_without_an_extra_map_update() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CameraPan>()
        .init_resource::<CurrentBgm>()
        .init_resource::<MemorizedBgm>();
    crate::world::saved::register(&mut app);
    crate::player::saved_camera::register(&mut app);
    crate::audio::saved::register(&mut app);
    let mut data = MapData::for_test(20, 15);
    data.map_id = 3;
    app.insert_resource(data);
    let hero = app
        .world_mut()
        .spawn((
            Player {
                tile_x: 8,
                tile_y: 7,
                dir: 2,
                frame: 3,
                charset: "Chara1".into(),
                index: 0,
            },
            Transform::default(),
            MoveQueue::default(),
            RouteStepper::default(),
        ))
        .id();
    let motion = crate::world::saved::hero::snapshot(app.world_mut()).unwrap();
    app.world_mut().get_mut::<Player>(hero).unwrap().frame = 0;
    let mut pan = CameraPan::default();
    pan.locked = true;
    pan.offset = Vec2::new(32.0, -48.0);
    let pan = pan.snapshot();
    let track = BgmTrack {
        name: "House".into(),
        volume: 0.7,
        speed: 1.1,
        fade_in: 0.4,
    };
    crate::world::saved::hero::prepare(app.world_mut(), 3, Some(motion.clone()));
    crate::player::saved_camera::prepare(app.world_mut(), 3, Some(pan.clone()));
    crate::audio::saved::prepare(
        app.world_mut(),
        3,
        Some(MusicState {
            current: Some(track.clone()),
            memorized: Some(track.clone()),
        }),
    );
    app.world_mut().write_message(MapChanged);
    app.world_mut().write_message(MapRebuilt);
    flush(app.world_mut());
    assert_eq!(
        crate::world::saved::hero::snapshot(app.world_mut()),
        Some(motion)
    );
    assert_eq!(app.world().resource::<CameraPan>().snapshot(), pan);
    assert_eq!(memorized(app.world_mut()), Some(track.clone()));
    let heard = app
        .world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect::<Vec<_>>();
    assert_eq!(heard, [AudioRequest::StopBgm, track.replay()]);
    app.world_mut().get_mut::<Player>(hero).unwrap().frame = 2;
    app.world_mut().resource_mut::<CameraPan>().locked = false;
    app.world_mut().insert_resource(MemorizedBgm::default());
    flush(app.world_mut());
    app.update();
    assert_eq!(app.world().get::<Player>(hero).unwrap().frame, 2);
    assert!(!app.world().resource::<CameraPan>().locked);
    assert!(memorized(app.world_mut()).is_none());
    assert!(app.world().resource::<Messages<AudioRequest>>().is_empty());
}

fn memorized(world: &mut World) -> Option<BgmTrack> {
    world
        .run_system_cached(|music: crate::audio::saved::Capture| {
            music.snapshot().unwrap().memorized
        })
        .unwrap()
}

#[derive(Component)]
struct Arrived;

#[derive(Resource, Default)]
struct Trace(Vec<Stage>);

#[test]
fn restoration_stages_apply_deferred_state_before_later_owners_read_it() {
    let mut app = App::new();
    app.init_resource::<Trace>();
    register(
        &mut app,
        Stage::Characters,
        |mut commands: Commands, mut trace: ResMut<Trace>| {
            assert_eq!(trace.0, [Stage::Reset]);
            commands.spawn(Arrived);
            trace.0.push(Stage::Characters);
        },
    );
    register(&mut app, Stage::Reset, |mut trace: ResMut<Trace>| {
        trace.0.push(Stage::Reset);
    });
    register(
        &mut app,
        Stage::State,
        |arrived: Query<&Arrived>, mut trace: ResMut<Trace>| {
            assert_eq!(arrived.iter().count(), 1);
            trace.0.push(Stage::State);
        },
    );
    flush(app.world_mut());
    assert_eq!(
        app.world().resource::<Trace>().0,
        [Stage::Reset, Stage::Characters, Stage::State]
    );
}
