use super::*;
use crate::world::saved::hero::{HeroState, snapshot};

fn tick(hero: &mut Player, state: &mut HeroState, data: &MapData, dt: f32) -> Vec<String> {
    let mut queue = state.motion.clone().into_queue();
    let effects = drive_route(
        hero,
        &mut queue,
        &mut state.route,
        (10, 10),
        dt,
        |_, _, _, _, _| true,
    )
    .effects;
    let moving = queue.busy();
    queue.advance(hero, data, dt);
    state
        .route
        .animation
        .advance(hero, state.route.speed(), moving, queue.jumping(), dt);
    state.frame = hero.frame;
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

fn copy_player(hero: &Player) -> Player {
    Player {
        tile_x: hero.tile_x,
        tile_y: hero.tile_y,
        dir: hero.dir,
        frame: hero.frame,
        charset: hero.charset.clone(),
        index: hero.index,
    }
}

#[test]
fn saved_hero_motion_and_animation_resume_without_replaying_route_commands_at_four_frame_rates() {
    for fps in [15, 30, 60, 144] {
        for jumping in [false, true] {
            let (mut app, path, entity) = hero_app(&format!("hero-continue-{fps}-{jumping}"));
            let mut expected = snapshot(app.world_mut()).unwrap();
            let mut hero = copy_player(app.world().get::<Player>(entity).unwrap());
            let mut program = vec![10001, 8, 0, 0, 32, 7];
            program.extend(if jumping {
                vec![24, 1, 1, 2, 25]
            } else {
                vec![1]
            });
            program.extend([23, 32, 8, 1]);
            expected
                .route
                .force_route(RouteStepper::from_move_event(&program));
            let data = MapData::for_test(20, 15);
            assert_eq!(
                tick(&mut hero, &mut expected, &data, 1.0 / 60.0),
                ["switch:7:true"]
            );
            assert!(expected.motion.clone().into_queue().busy());
            assert_eq!(expected.motion.clone().into_queue().jumping(), jumping);
            app.world_mut().entity_mut(entity).insert((
                copy_player(&hero),
                expected.motion.clone().into_queue(),
                expected.route.clone(),
            ));
            save_and_load(&mut app);
            std::fs::remove_file(path).unwrap();
            let mut actual = snapshot(app.world_mut()).unwrap();
            let mut loaded_hero = copy_player(app.world().get::<Player>(entity).unwrap());
            assert_eq!(actual, expected);
            let mut effects = Vec::new();
            for _ in 0..fps * 4 {
                let got = tick(&mut loaded_hero, &mut actual, &data, 1.0 / fps as f32);
                assert_eq!(got, tick(&mut hero, &mut expected, &data, 1.0 / fps as f32));
                assert_eq!(actual, expected, "{fps} FPS, jumping={jumping}");
                assert_eq!(
                    (loaded_hero.tile(), loaded_hero.dir),
                    (hero.tile(), hero.dir)
                );
                effects.extend(got);
            }
            assert_eq!(effects, ["switch:8:true"]);
            assert!(!actual.route.pending() && !actual.route.forced());
            assert!(!actual.motion.clone().into_queue().busy());
            assert_eq!(hero.tile(), if jumping { (13, 11) } else { (12, 10) });
        }
    }
}
