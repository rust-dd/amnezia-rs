use super::*;
use crate::player::{CameraPan, Player};
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::{Fade, PendingTeleport, TeleportPlugin};
use crate::world::{MapChanged, MapData, MapEvents, MoveQueue, RouteStepper};

fn transfer_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        crate::transitions::TransitionPlugin,
        TeleportPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<Switches>()
    .init_resource::<Variables>()
    .init_resource::<Party>()
    .init_resource::<Inventory>()
    .init_resource::<CameraPan>()
    .init_resource::<MapEvents>()
    .add_message::<MapChanged>()
    .add_systems(
        Update,
        clear_on_map_change.after(crate::teleport::MapTransfer),
    );
    let mut map = MapData::for_test(20, 15);
    map.map_id = 3;
    app.insert_resource(map);
    let hero = app
        .world_mut()
        .spawn((
            Player {
                tile_x: 15,
                tile_y: 12,
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

fn finish_transfer(app: &mut App) {
    for frame in 0..80 {
        app.world_mut()
            .resource_mut::<crate::timing::GameFrames>()
            .frame = frame;
        app.update();
    }
    assert!(!app.world().resource::<Fade>().busy());
}

#[test]
fn same_map_teleports_keep_screen_pictures_and_their_map_fixed_anchors() {
    let (mut app, hero) = transfer_app();
    let first = app.world_mut().spawn(test_picture(1)).id();
    let mut fixed = test_picture(2);
    fixed.fixed_to_map = true;
    fixed.world_anchor = Some(Vec2::new(10.0, 20.0));
    let second = app.world_mut().spawn(fixed).id();
    app.world_mut().resource_mut::<PendingTeleport>().0 = Some((3, 8, 7));
    finish_transfer(&mut app);
    let player = app.world().get::<Player>(hero).unwrap();
    assert_eq!((player.tile_x, player.tile_y), (8, 7));
    assert!(app.world().get::<Picture>(first).is_some());
    assert_eq!(
        app.world().get::<Picture>(second).unwrap().world_anchor,
        Some(Vec2::new(10.0, 20.0))
    );
}

#[test]
fn cross_map_transfers_and_forced_reloads_clear_the_previous_scene_pictures() {
    for (map_id, reload) in [(2, false), (3, true)] {
        let (mut app, _) = transfer_app();
        let old = app.world_mut().spawn(test_picture(1)).id();
        if reload {
            app.world_mut()
                .resource_mut::<PendingTeleport>()
                .reload(map_id, 8, 7);
        } else {
            app.world_mut().resource_mut::<PendingTeleport>().0 = Some((map_id, 8, 7));
        }
        finish_transfer(&mut app);
        assert_eq!(app.world().resource::<MapData>().map_id, map_id);
        assert!(app.world().get::<Picture>(old).is_none());
        assert_eq!(count_pictures(&mut app), 0);
    }
}

#[test]
fn every_transfer_announces_arrival_but_only_rebuilt_maps_announce_a_new_scene() {
    #[derive(Resource, Default)]
    struct Counts {
        arrived: u32,
        rebuilt: u32,
    }
    for (map_id, reload, rebuilt) in [(3, false, 0), (2, false, 1), (3, true, 1)] {
        let (mut app, _) = transfer_app();
        app.init_resource::<Counts>().add_systems(
            Update,
            (|mut arrived: MessageReader<MapChanged>,
              mut rebuilt: MessageReader<MapRebuilt>,
              mut counts: ResMut<Counts>| {
                counts.arrived += arrived.read().count() as u32;
                counts.rebuilt += rebuilt.read().count() as u32;
            })
            .after(crate::teleport::MapTransfer),
        );
        if reload {
            app.world_mut()
                .resource_mut::<PendingTeleport>()
                .reload(map_id, 8, 7);
        } else {
            app.world_mut().resource_mut::<PendingTeleport>().0 = Some((map_id, 8, 7));
        }
        finish_transfer(&mut app);
        let counts = app.world().resource::<Counts>();
        assert_eq!((counts.arrived, counts.rebuilt), (1, rebuilt));
    }
}
