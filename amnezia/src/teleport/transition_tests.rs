use super::*;
use crate::timing::GameFrames;
use crate::transitions::{Defaults, Settings, Transition, TransitionPlugin};

fn app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        TransitionPlugin,
        TeleportPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<Switches>()
    .init_resource::<Variables>()
    .init_resource::<Party>()
    .init_resource::<Inventory>()
    .init_resource::<CameraPan>()
    .init_resource::<MapEvents>()
    .add_message::<MapChanged>();
    let mut map = MapData::for_test(20, 15);
    map.map_id = 3;
    app.insert_resource(map);
    let hero = app
        .world_mut()
        .spawn((
            Player {
                tile_x: 1,
                tile_y: 2,
                dir: 2,
                frame: 1,
                charset: "Chara1".into(),
                index: 0,
            },
            Transform::default(),
            MoveQueue::default(),
            RouteStepper::default(),
        ))
        .id();
    (app, hero)
}

fn tick(app: &mut App, frame: u32) {
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
}

#[test]
fn none_out_and_cut_in_reposition_without_a_black_fade() {
    let (mut app, hero) = app();
    let defaults = Defaults([0; 6]);
    let mut settings = app.world_mut().resource_mut::<Settings>();
    settings.change(&[0, 20], &defaults);
    settings.change(&[1, 19], &defaults);
    app.world_mut().resource_mut::<PendingTeleport>().0 = Some((3, 4, 6));
    tick(&mut app, 0);
    assert!(!app.world().resource::<Transition>().busy());
    assert!(!app.world().resource::<Transition>().erased());
    tick(&mut app, 0);
    let player = app.world().get::<Player>(hero).unwrap();
    assert_eq!((player.tile_x, player.tile_y), (4, 6));
    tick(&mut app, 0);
    assert!(app.world().resource::<Transition>().busy());
    tick(&mut app, 1);
    assert!(!app.world().resource::<Fade>().busy());
    assert!(!app.world().resource::<Transition>().erased());
}

#[test]
fn an_event_erased_screen_stays_black_across_the_transfer() {
    let (mut app, hero) = app();
    app.world_mut()
        .resource_mut::<Transition>()
        .start(Kind::Cut, true, 0, IVec2::ZERO);
    app.world_mut().resource_mut::<Transition>().event_erased = true;
    tick(&mut app, 1);
    assert!(app.world().resource::<Transition>().erased());
    app.world_mut().resource_mut::<PendingTeleport>().0 = Some((3, 8, 7));
    for _ in 0..4 {
        tick(&mut app, 1);
    }
    let player = app.world().get::<Player>(hero).unwrap();
    assert_eq!((player.tile_x, player.tile_y), (8, 7));
    assert!(!app.world().resource::<Fade>().busy());
    assert!(app.world().resource::<Transition>().erased());
}

#[test]
fn default_transfer_swaps_only_after_erase_then_waits_for_show() {
    let (mut app, hero) = app();
    app.world_mut().resource_mut::<PendingTeleport>().0 = Some((3, 8, 7));
    tick(&mut app, 0);
    tick(&mut app, 34);
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile_x, 1);
    tick(&mut app, 35);
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile_x, 8);
    assert!(app.world().resource::<Transition>().erased());
    tick(&mut app, 36);
    tick(&mut app, 70);
    assert!(app.world().resource::<Fade>().busy());
    tick(&mut app, 71);
    assert!(!app.world().resource::<Fade>().busy());
    assert!(!app.world().resource::<Transition>().erased());
}

#[test]
fn a_map_transfer_keeps_the_route_and_resets_animation_only_when_the_map_changes() {
    for map_id in [3, 4] {
        let (mut app, hero) = app();
        let mut route = RouteStepper::from_move_event(&[10001, 4, 1, 0, 1, 32, 9]).with_speed(2);
        {
            let mut player = app.world_mut().get_mut::<Player>(hero).unwrap();
            player.frame = 3;
            route
                .animation
                .advance(&mut *player, 2, true, false, 0.0625);
        }
        app.world_mut().entity_mut(hero).insert(route.clone());
        app.world_mut().resource_mut::<PendingTeleport>().0 = Some((map_id, 8, 7));
        tick(&mut app, 0);
        tick(&mut app, 35);
        let player = app.world().get::<Player>(hero).unwrap();
        assert_eq!(player.frame, if map_id == 3 { 3 } else { 1 });
        assert_eq!(player.tile(), (8, 7));
        if map_id != 3 {
            route.animation = ron::from_str("(mode:0,paused:false,count:0,fraction:0.75)").unwrap();
        }
        assert_eq!(app.world().get::<RouteStepper>(hero).unwrap(), &route);
    }
}
