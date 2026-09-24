use super::*;
use crate::teleport::TeleportPlugin;
use crate::world::{MapData, MapEvents, MapScene};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[test]
fn new_game_reset_keeps_the_title_erased_until_the_start_map_is_ready() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        crate::transitions::TransitionPlugin,
        SessionPlugin,
    ))
    .init_resource::<PendingTeleport>();
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .hold_black();
    app.world_mut().resource_mut::<NewGameRequest>().requested = true;
    app.update();
    assert!(
        app.world()
            .resource::<crate::transitions::Transition>()
            .erased()
    );
    assert_eq!(app.world().resource::<PendingTeleport>().0, Some((5, 0, 0)));
}

#[test]
fn new_game_clears_progress_and_rebuilds_even_the_same_start_map() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        crate::timing::TimingPlugin,
        crate::transitions::TransitionPlugin,
        TeleportPlugin,
        SessionPlugin,
    ))
    .init_asset::<Image>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        1.0 / 60.0,
    )))
    .init_resource::<Switches>()
    .init_resource::<Variables>()
    .init_resource::<Party>()
    .init_resource::<Inventory>()
    .init_resource::<CameraPan>()
    .init_resource::<MapEvents>()
    .add_message::<crate::world::MapChanged>();
    let mut map = MapData::for_test(20, 15);
    map.map_id = 5;
    app.insert_resource(map);
    app.world_mut().resource_mut::<Switches>().set(8, true);
    app.world_mut().resource_mut::<Inventory>().add_gold(999);
    let mut frames = crate::timing::GameFrames::default();
    frames.advance(100.0);
    app.insert_resource(frames);
    app.world_mut()
        .resource_mut::<crate::transitions::Settings>()
        .change(&[0, 20], &crate::transitions::Defaults([0; 6]));
    let old_scene = app.world_mut().spawn(MapScene).id();
    let hero = app
        .world_mut()
        .spawn((
            Player {
                tile_x: 8,
                tile_y: 10,
                dir: 1,
                frame: 0,
                charset: "Poses2".into(),
                index: 4,
            },
            Transform::default(),
            MoveQueue::default(),
            RouteStepper::default(),
        ))
        .id();
    app.world_mut().resource_mut::<NewGameRequest>().requested = true;
    app.update();
    assert_eq!(app.world().resource::<crate::timing::GameFrames>().frame, 0);
    for _ in 1..80 {
        app.update();
    }
    assert!(!app.world().resource::<Switches>().get(8));
    assert_eq!(app.world().resource::<Inventory>().gold(), 0);
    assert_eq!(
        app.world().resource::<crate::timing::GameFrames>().frame,
        79
    );
    assert!(!app.world().resource::<RunningEvent>().active());
    assert_eq!(
        *app.world().resource::<crate::transitions::Settings>(),
        crate::transitions::Settings::default()
    );
    assert!(app.world().get_entity(old_scene).is_err());
    assert!(!app.world().resource::<MapEvents>().events.is_empty());
    let player = app.world().get::<Player>(hero).unwrap();
    assert_eq!(
        (player.tile_x, player.tile_y, player.charset.as_str()),
        (0, 0, "Chara1")
    );
    assert!(!app.world().resource::<crate::teleport::Fade>().busy());
}

#[test]
fn clearing_a_session_stops_the_previous_script_and_movement() {
    let mut world = World::new();
    let mut event = RunningEvent::default();
    event.start(10, vec![]);
    world.insert_resource(event);
    let mut queue = MoveQueue::default();
    queue.push_step(crate::world::RouteAction::Step {
        dx: 1,
        dy: 0,
        face: 1,
    });
    let mover = world.spawn((queue, RouteStepper::default())).id();
    clear_transient(&mut world);
    assert!(!world.resource::<RunningEvent>().active());
    assert!(!world.get::<MoveQueue>(mover).unwrap().busy());
}

#[test]
fn clearing_a_session_discards_pending_character_unpause_requests() {
    use crate::interpreter::foreground::UnpauseEvent;
    let mut world = World::new();
    world.init_resource::<Messages<UnpauseEvent>>();
    world.write_message(UnpauseEvent(1));
    clear_transient(&mut world);
    assert!(world.resource::<Messages<UnpauseEvent>>().is_empty());
}
