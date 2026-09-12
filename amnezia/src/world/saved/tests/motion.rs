use super::*;
use amnezia_data::{MoveCommandDef, MoveRouteDef};

fn tick(state: &mut EventState, data: &MapData, dt: f32) -> Vec<String> {
    let mut queue = state.motion.clone().into_queue();
    let effects = drive_route(
        &mut state.character,
        &mut queue,
        &mut state.route,
        (10, 10),
        dt,
        |_, _, _, _, _| true,
    )
    .effects;
    let moving = queue.busy();
    queue.advance(&mut state.character, data, dt);
    let speed = state.route.speed();
    state
        .route
        .animation
        .advance(&mut state.character, speed, moving, queue.jumping(), dt);
    state.motion = queue.snapshot();
    effects
        .into_iter()
        .map(|effect| match effect {
            StepEffect::Switch(id, on) => format!("switch:{id}:{on}"),
            StepEffect::Sound { name, params } => format!("sound:{name}:{params:?}"),
            StepEffect::Transparency(level) => format!("alpha:{level}"),
        })
        .collect()
}

#[test]
fn a_saved_jump_and_suspended_page_route_continue_without_replaying_effects() {
    for fps in [15, 30, 60, 144] {
        let (mut app, path) = app(&format!("jump-{fps}"));
        let mut expected = snapshot(app.world_mut()).remove(0);
        expected.character.tile_x = 7;
        expected.character.tile_y = 8;
        expected.autonomy = ron::from_str::<AutoMove>(
            "(move_type:1,frequency:4,speed:2,timer:0.731,rng:123456789)",
        )
        .unwrap();
        expected.route = RouteStepper::from_page(
            &MoveRouteDef {
                commands: vec![32, 23, 1, 32]
                    .into_iter()
                    .enumerate()
                    .map(|(i, code)| MoveCommandDef {
                        code,
                        params: vec![9100 + i as i32],
                        string: String::new(),
                    })
                    .collect(),
                repeat: false,
                skippable: false,
            },
            4,
            8,
        );
        let data = MapData::for_test(20, 15);
        assert_eq!(tick(&mut expected, &data, 1.0 / 60.0), ["switch:9100:true"]);
        expected.route.force_route(RouteStepper::from_move_event(&[
            1, 8, 0, 0, 36, 40, 26, 32, 7, 24, 1, 1, 2, 25, 23, 27, 37, 41, 32, 8,
        ]));
        let effects = tick(&mut expected, &data, 1.0 / 60.0);
        assert!(effects.contains(&"switch:7:true".to_owned()));
        assert!(expected.motion.clone().into_queue().jumping());
        let entity = npc(app.world_mut());
        app.world_mut().entity_mut(entity).insert((
            expected.character.clone(),
            expected.motion.clone().into_queue(),
            expected.route.clone(),
            expected.autonomy.clone(),
            expected.page.clone(),
        ));
        save_and_load(&mut app);
        std::fs::remove_file(path).unwrap();
        let mut actual = snapshot(app.world_mut()).remove(0);
        assert_eq!(actual, expected);
        let mut effects = Vec::new();
        for _ in 0..fps * 4 {
            let got = tick(&mut actual, &data, 1.0 / fps as f32);
            assert_eq!(got, tick(&mut expected, &data, 1.0 / fps as f32));
            assert_eq!(actual, expected, "{fps} FPS");
            effects.extend(got);
        }
        assert!(!actual.route.active() && !actual.route.pending());
        assert!(!actual.motion.clone().into_queue().busy());
        assert_eq!(
            effects
                .iter()
                .filter(|effect| effect.starts_with("switch:"))
                .cloned()
                .collect::<Vec<_>>(),
            ["switch:8:true", "switch:9103:true"]
        );
        assert_eq!((actual.character.tile_x, actual.character.tile_y), (10, 9));
    }
}

#[test]
fn a_loop_seam_and_pending_steps_keep_their_unwrapped_motion() {
    let (mut app, path) = app("loop");
    let mut expected = snapshot(app.world_mut()).remove(0);
    let mut data = MapData::for_test(20, 15);
    data.scroll_type = 3;
    expected.character.tile_x = 0;
    expected.character.tile_y = 0;
    let mut queue = MoveQueue::default();
    queue.enqueue_route([
        RouteAction::Step {
            dx: -1,
            dy: 0,
            face: 3,
        },
        RouteAction::Step {
            dx: 0,
            dy: -1,
            face: 0,
        },
    ]);
    queue.advance(&mut expected.character, &data, 0.04);
    expected.motion = queue.snapshot();
    assert_eq!(expected.character.tile_x, 19);
    assert!(queue.render_position(&expected.character, &data).x < -152.0);
    let encoded = ron::to_string(&expected).unwrap();
    let mut actual = ron::from_str::<EventState>(&encoded).unwrap();
    for _ in 0..30 {
        assert_eq!(
            tick(&mut actual, &data, 1.0 / 60.0),
            tick(&mut expected, &data, 1.0 / 60.0)
        );
        assert_eq!(actual, expected);
    }
    assert_eq!((actual.character.tile_x, actual.character.tile_y), (19, 14));
    assert!(!path.exists());
}
