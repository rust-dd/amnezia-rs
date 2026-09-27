use super::*;

fn copy(vehicles: &Vehicles) -> Vehicles {
    Vehicles {
        save: vehicles.save.clone(),
        motion: vehicles.motion.clone(),
        ..default()
    }
}

fn tick(vehicles: &mut Vehicles, data: &MapData, dt: f32) -> Vec<String> {
    let mut effects = Vec::new();
    for (index, (vehicle, motion)) in vehicles
        .save
        .vehicles
        .iter_mut()
        .zip(&mut vehicles.motion)
        .enumerate()
    {
        let routed = motion.route.active();
        let driven = drive_route(
            vehicle,
            &mut motion.queue,
            &mut motion.route,
            (20, 20),
            |_, _, _, _, _| true,
        );
        if routed {
            vehicle.speed = motion.route.speed();
        }
        for effect in driven.effects {
            effects.push(match effect {
                StepEffect::Switch(id, on) => format!("vehicle:{index}:switch:{id}:{on}"),
                StepEffect::Sound { name, params } => {
                    format!("vehicle:{index}:sound:{name}:{params:?}")
                }
                StepEffect::Transparency(level) => {
                    motion.alpha = crate::tiles::character_alpha(level);
                    format!("vehicle:{index}:alpha:{level}")
                }
            });
        }
        let moving = motion.queue.busy();
        if let Some(pixel) = motion.queue.advance(vehicle, data, dt) {
            motion.pixel = Some(pixel);
        }
        if moving && !motion.queue.busy() {
            motion.route.settle_movement();
        }
        motion.route.advance_stop_clock(moving, true);
        let animated = !motion.queue.jumping() && (index != 2 || vehicles.save.riding == Some(2));
        motion
            .route
            .animation
            .advance_vehicle(vehicle, animated, moving, dt);
    }
    effects
}

#[test]
fn all_vehicle_routes_and_animation_clocks_resume_without_repeated_commands_at_four_frame_rates() {
    for fps in [15, 30, 60, 144] {
        let (mut app, path) = app(&format!("continue-{fps}"));
        app.world_mut().resource_scope(|world, data: Mut<MapData>| {
            let mut vehicles = world.resource_mut::<Vehicles>();
            for index in 0..3 {
                let mut commands = vec![32, 7];
                commands.extend(if index == 1 {
                    vec![24, 1, 1, 2, 25]
                } else {
                    vec![1]
                });
                commands.extend([23, 27, 41, 37, 32, 8, 1]);
                start(&mut vehicles, &data, index, &commands);
            }
            vehicles.save.riding = Some(2);
        });
        app.update();
        let mut expected = copy(app.world().resource::<Vehicles>());
        save_and_load(&mut app);
        std::fs::remove_file(path).unwrap();
        let mut actual = copy(app.world().resource::<Vehicles>());
        assert_eq!(actual.motion_snapshot(), expected.motion_snapshot());
        let data = app.world().resource::<MapData>();
        let mut effects = Vec::new();
        let mut clock = crate::timing::GameFrames::default();
        for _ in 0..fps * 4 {
            let before = clock.frame;
            clock.advance(1.0 / f64::from(fps));
            for _ in before..clock.frame {
                let got = tick(&mut actual, data, 1.0 / 60.0);
                assert_eq!(got, tick(&mut expected, data, 1.0 / 60.0));
                assert_eq!(
                    actual.motion_snapshot(),
                    expected.motion_snapshot(),
                    "{fps} FPS"
                );
                assert_eq!(actual.save, expected.save, "{fps} FPS");
                effects.extend(got);
            }
        }
        assert!(!actual.routes_pending());
        let mut switches = effects
            .into_iter()
            .filter(|effect| effect.contains(":switch:"))
            .collect::<Vec<_>>();
        switches.sort();
        assert_eq!(
            switches,
            [
                "vehicle:0:switch:8:true",
                "vehicle:1:switch:8:true",
                "vehicle:2:switch:8:true"
            ]
        );
        for (index, tile) in [(22, 20), (31, 21), (38, 20)].into_iter().enumerate() {
            assert_eq!(actual.save.vehicles[index].tile(), tile);
            assert!(!actual.motion[index].queue.busy());
            assert_eq!(actual.motion[index].alpha, 1.0);
        }
    }
}
