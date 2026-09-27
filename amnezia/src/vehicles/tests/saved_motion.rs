use super::*;
use crate::save::{EventSaveRequest, LoadOutcome, LoadRequest, SavePlugin};

mod continuation;
mod lifecycle;
mod stops;

fn app(tag: &str) -> (App, std::path::PathBuf) {
    let path =
        std::env::temp_dir().join(format!("amnezia-vehicle-{tag}-{}.ron", std::process::id()));
    let mut app = crate::save::tests::save_resources(path.clone());
    app.add_plugins((AssetPlugin::default(), SavePlugin))
        .init_asset::<Image>()
        .init_resource::<Vehicles>()
        .init_resource::<crate::interpreter::RunningEvent>()
        .init_resource::<MapEvents>()
        .add_systems(Update, rebuild.in_set(crate::teleport::MapTransfer))
        .add_systems(PostUpdate, render::sync_hero);
    let mut data = MapData::for_test(140, 140);
    data.map_id = 13;
    app.insert_resource(data);
    app.world_mut().spawn((
        Player {
            tile_x: 20,
            tile_y: 20,
            dir: 2,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
        },
        MoveQueue::default(),
        crate::world::RouteStepper::default(),
        Sprite::default(),
        Transform::default(),
    ));
    app.update();
    (app, path)
}

fn rebuild(world: &mut World) {
    let Some((map_id, x, y)) = world
        .resource_mut::<crate::teleport::PendingTeleport>()
        .0
        .take()
    else {
        return;
    };
    let mut data = MapData::for_test(140, 140);
    data.map_id = map_id;
    let (wx, wy) = data.tile_center(x as i32, y as i32);
    world.insert_resource(data);
    let (mut hero, mut queue, mut route, mut transform) = world
        .query::<(
            &mut Player,
            &mut MoveQueue,
            &mut crate::world::RouteStepper,
            &mut Transform,
        )>()
        .single_mut(world)
        .unwrap();
    hero.tile_x = x as i32;
    hero.tile_y = y as i32;
    *queue = default();
    *route = default();
    transform.translation = Vec3::new(wx, wy + hero.y_offset(), hero.draw_z(y as i32));
    world.write_message(crate::world::MapRebuilt);
    world.write_message(crate::world::MapChanged);
}

fn save_and_load(app: &mut App) {
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
}

fn start(vehicles: &mut Vehicles, data: &MapData, index: usize, commands: &[i32]) {
    vehicles.set_location(index, data.map_id, 20 + index as u32 * 8, 20);
    let mut program = vec![10002 + index as i32, 8, 0, 0, 36, 26, 40];
    program.extend(commands);
    vehicles.set_route(
        10002 + index as i32,
        crate::world::RouteStepper::from_move_event(&program),
    );
    let vehicle = &mut vehicles.save.vehicles[index];
    vehicle.set_graphic("Chara1".into(), 2);
    let motion = &mut vehicles.motion[index];
    for effect in drive_route(
        vehicle,
        &mut motion.queue,
        &mut motion.route,
        (20, 20),
        |_, _, _, _, _| true,
    )
    .effects
    {
        if let StepEffect::Transparency(level) = effect {
            motion.alpha = crate::tiles::character_alpha(level);
        }
    }
    vehicle.speed = motion.route.speed();
    motion.pixel = motion.queue.advance(vehicle, data, 0.04);
    motion.route.animation.paused = true;
    vehicle.frame = 2;
}

#[test]
fn loading_keeps_all_three_vehicles_in_progress_steps_jumps_and_route_parameters() {
    let (mut app, path) = app("all-motion");
    app.world_mut().resource_scope(|world, data: Mut<MapData>| {
        let mut vehicles = world.resource_mut::<Vehicles>();
        for index in 0..3 {
            start(
                &mut vehicles,
                &data,
                index,
                if index == 1 {
                    &[24, 1, 1, 2, 25, 23, 1]
                } else {
                    &[1, 23, 2]
                },
            );
        }
    });
    let base = app.world().resource::<Vehicles>().save.clone();
    let expected = app
        .world()
        .resource::<Vehicles>()
        .motion
        .iter()
        .map(|motion| {
            (
                motion.queue.snapshot(),
                motion.route.clone(),
                motion.pixel,
                motion.alpha,
            )
        })
        .collect::<Vec<_>>();
    save_and_load(&mut app);
    std::fs::remove_file(path).unwrap();
    let vehicles = app.world().resource::<Vehicles>();
    assert_eq!(vehicles.save, base);
    for (motion, expected) in vehicles.motion.iter().zip(expected) {
        assert_eq!(
            (
                motion.queue.snapshot(),
                motion.route.clone(),
                motion.pixel,
                motion.alpha
            ),
            expected
        );
    }
}

#[test]
fn loading_a_boarded_airship_restores_the_riders_mid_step_position_before_camera_sync() {
    let (mut app, path) = app("riding-motion");
    app.world_mut().resource_scope(|world, data: Mut<MapData>| {
        let mut vehicles = world.resource_mut::<Vehicles>();
        start(&mut vehicles, &data, 2, &[1, 23, 1]);
        vehicles.save.riding = Some(2);
    });
    app.update();
    let expected = app
        .world_mut()
        .query_filtered::<&Transform, With<Player>>()
        .single(app.world())
        .unwrap()
        .translation
        .truncate();
    save_and_load(&mut app);
    std::fs::remove_file(path).unwrap();
    assert_eq!(
        app.world_mut()
            .query_filtered::<&Transform, With<Player>>()
            .single(app.world())
            .unwrap()
            .translation
            .truncate(),
        expected
    );
    assert!(app.world().resource::<Vehicles>().motion[2].queue.busy());
    assert!(app.world().resource::<Vehicles>().riding());
}

#[test]
fn loading_preserves_a_vehicle_wait_even_without_an_active_tween() {
    let (mut app, path) = app("route-wait");
    app.world_mut().resource_scope(|world, data: Mut<MapData>| {
        start(&mut world.resource_mut::<Vehicles>(), &data, 0, &[23, 1]);
    });
    let expected = app.world().resource::<Vehicles>().motion[0].route.clone();
    assert!(expected.pending());
    assert!(!app.world().resource::<Vehicles>().motion[0].queue.busy());
    save_and_load(&mut app);
    std::fs::remove_file(path).unwrap();
    assert_eq!(app.world().resource::<Vehicles>().motion[0].route, expected);
}
