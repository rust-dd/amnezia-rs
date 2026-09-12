use super::*;
use crate::save::{EventSaveRequest, LoadOutcome, LoadRequest, SavePlugin};

mod hero;
mod lifecycle;
mod motion;
mod validation;

fn app(tag: &str) -> (App, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("amnezia-npc-{tag}-{}.ron", std::process::id()));
    let mut app = crate::save::tests::save_resources(path.clone());
    app.add_plugins((AssetPlugin::default(), SavePlugin))
        .init_asset::<Image>()
        .init_resource::<crate::interpreter::RunningEvent>()
        .add_message::<MapRebuilt>()
        .insert_resource(pages::EventTileset(Handle::default()))
        .add_systems(Startup, spawn)
        .add_systems(Update, rebuild.in_set(crate::teleport::MapTransfer));
    let map = load_ron::<Map>(&format!("{}/maps/map_0003.ron", asset_root()));
    let mut data = MapData::for_test(map.width as i32, map.height as i32);
    data.map_id = 3;
    app.insert_resource(data).insert_resource(MapEvents {
        events: vec![map.events[0].clone()],
    });
    app.world_mut().spawn(crate::player::Player {
        tile_x: 10,
        tile_y: 10,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    app.update();
    (app, path)
}

#[allow(clippy::too_many_arguments)]
fn spawn(
    mut commands: Commands,
    server: Res<AssetServer>,
    data: Res<MapData>,
    events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
) {
    pages::spawn_event(
        &mut commands,
        &server,
        (&switches, &variables, &party, &inventory),
        &events.events[0],
        (data.offset_x, data.offset_y),
        &Handle::default(),
    );
}

fn rebuild(world: &mut World) {
    use bevy::ecs::system::RunSystemOnce;
    if world.resource::<LoadOutcome>().0 != Some(true)
        || world
            .resource::<crate::teleport::PendingTeleport>()
            .0
            .is_none()
    {
        return;
    }
    let (_, x, y) = world
        .resource_mut::<crate::teleport::PendingTeleport>()
        .0
        .take()
        .unwrap();
    let (wx, wy) = world.resource::<MapData>().tile_center(x as i32, y as i32);
    for (mut hero, queue, route, transform) in world
        .query::<(
            &mut crate::player::Player,
            Option<&mut MoveQueue>,
            Option<&mut RouteStepper>,
            Option<&mut Transform>,
        )>()
        .iter_mut(world)
    {
        hero.tile_x = x as i32;
        hero.tile_y = y as i32;
        if let Some(mut queue) = queue {
            *queue = default();
        }
        if let Some(mut route) = route {
            *route = default();
        }
        if let Some(mut transform) = transform {
            transform.translation = Vec3::new(wx, wy + hero.y_offset(), hero.draw_z(y as i32));
        }
    }
    let entities = world
        .query_filtered::<Entity, With<EventSprite>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in entities {
        world.despawn(entity);
    }
    let map = load_ron::<Map>(&format!("{}/maps/map_0003.ron", asset_root()));
    world.resource_mut::<MapEvents>().events = vec![map.events[0].clone()];
    world.run_system_once(spawn).unwrap();
    world.write_message(MapRebuilt);
    world.write_message(MapChanged);
}

fn npc(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<EventSprite>>()
        .single(world)
        .unwrap()
}

fn save_and_load(app: &mut App) {
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
}

#[test]
fn reloading_restores_the_npcs_live_tile_pose_and_graphic() {
    let (mut app, path) = app("position");
    let entity = npc(app.world_mut());
    {
        let mut event = app.world_mut().get_mut::<EventSprite>(entity).unwrap();
        event.tile_x = 7;
        event.tile_y = 8;
        event.dir = 3;
        event.frame = 2;
        event.charset = "Poses2".into();
        event.index = 4;
    }
    save_and_load(&mut app);
    std::fs::remove_file(path).unwrap();
    let entity = npc(app.world_mut());
    let event = app.world().get::<EventSprite>(entity).unwrap();
    assert_eq!(
        (event.tile_x, event.tile_y, event.dir, event.frame),
        (7, 8, 3, 2)
    );
    assert_eq!((event.charset.as_str(), event.index), ("Poses2", 4));
    let logical = &app.world().resource::<MapEvents>().events[0];
    assert_eq!((logical.x, logical.y), (7, 8));
    let (x, y) = app.world().resource::<MapData>().tile_center(7, 8);
    let rendered = app.world().get::<Transform>(entity).unwrap().translation;
    assert_eq!(
        rendered,
        Vec3::new(x, y + event.y_offset(), event.draw_z(8))
    );
}

#[test]
fn reloading_keeps_an_npcs_in_progress_step_and_forced_route() {
    let (mut app, path) = app("motion");
    app.world_mut().resource_scope(|world, data: Mut<MapData>| {
        let (mut event, mut queue, mut route) = world
            .query::<(&mut EventSprite, &mut MoveQueue, &mut RouteStepper)>()
            .single_mut(world)
            .unwrap();
        route.force_route(RouteStepper::from_move_event(&[
            1, 8, 0, 0, 36, 40, 26, 1, 23, 2,
        ]));
        drive_route(
            &mut *event,
            &mut queue,
            &mut route,
            (10, 10),
            1.0 / 60.0,
            |_, _, _, _, _| true,
        );
        queue.advance(&mut *event, &data, 0.04);
        assert!(queue.busy() && route.forced());
    });
    let entity = npc(app.world_mut());
    let expected = {
        let world = app.world();
        world.get::<MoveQueue>(entity).unwrap().render_position(
            world.get::<EventSprite>(entity).unwrap(),
            world.resource::<MapData>(),
        )
    };
    save_and_load(&mut app);
    std::fs::remove_file(path).unwrap();
    let entity = npc(app.world_mut());
    let world = app.world();
    let queue = world.get::<MoveQueue>(entity).unwrap();
    assert!(queue.busy());
    let route = world.get::<RouteStepper>(entity).unwrap();
    assert!(route.forced() && route.pending() && route.through());
    assert_eq!(route.alpha(), crate::tiles::character_alpha(1));
    assert_eq!(
        queue.render_position(
            world.get::<EventSprite>(entity).unwrap(),
            world.resource::<MapData>()
        ),
        expected
    );
    assert_eq!(
        world.get::<Sprite>(entity).unwrap().color.alpha(),
        route.alpha()
    );
}
