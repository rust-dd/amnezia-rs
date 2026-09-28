use super::*;
use crate::timing::{
    GameFrames, SceneFrames, TimingPlugin,
    logical::{LogicalPlugin, Step},
};
use crate::transitions::TransitionPlugin;

#[derive(Debug)]
struct Sample {
    raw: u32,
    scene: u32,
    callback: bool,
    hero: Transform,
    hero_x: i32,
    npc: Transform,
    tone: f32,
}

#[derive(Resource, Default)]
struct Trace(Vec<Sample>);

#[derive(Resource)]
struct ObservedNpc(Entity);

fn record(world: &mut World) {
    let Some(npc) = world.get_resource::<ObservedNpc>() else {
        return;
    };
    let npc = *world.get::<Transform>(npc.0).unwrap();
    let (hero, transform) = world
        .query::<(&Player, &Transform)>()
        .single(world)
        .unwrap();
    let sample = Sample {
        raw: world.resource::<GameFrames>().frame,
        scene: world.resource::<SceneFrames>().frame,
        callback: world.resource::<Step>().callback,
        hero: *transform,
        hero_x: hero.tile_x,
        npc,
        tone: world.resource::<crate::screenfx::TintState>().tone()[0],
    };
    world.resource_mut::<Trace>().0.push(sample);
}

#[test]
fn callbacks_finish_only_owed_character_and_effect_stages_before_the_fresh_update() {
    for foreground in [false, true] {
        for fps in [15, 30, 60, 144] {
            let mut app = unstarted_app();
            app.add_plugins((
                TimingPlugin,
                LogicalPlugin,
                TransitionPlugin,
                crate::screenfx::ScreenFxPlugin,
            ))
            .init_resource::<Trace>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .add_systems(
                Update,
                record
                    .after(crate::interpreter::InterpreterStep)
                    .after(crate::screenfx::ScreenEffectsSet),
            );
            app.update();
            let first = npcs::npc(&mut app, 1, 1, vec![], &[1, 8, 0, 0, 1]);
            app.insert_resource(ObservedNpc(first));
            let commands = vec![
                cmd(11010, 0, vec![0]),
                switch_cmd(70, 0, 0),
                cmd(11410, 0, vec![100]),
            ];
            if foreground {
                app.world_mut()
                    .resource_mut::<RunningEvent>()
                    .start(7, commands);
            } else {
                npcs::npc(&mut app, 2, 3, commands, &[2, 8, 0, 0, 3]);
            }
            queue_hero_step(app.world_mut(), crate::tiles::DIR_RIGHT, 4);
            app.world_mut().write_message(ScreenEffect::Tint {
                r: 40,
                g: 100,
                b: 100,
                sat: 100,
                secs: 1.0,
            });
            app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1.0 / f64::from(fps),
            )));
            for _ in 0..fps {
                app.update();
            }
            let trace = &app.world().resource::<Trace>().0;
            let first = &trace[0];
            assert_eq!(first.hero_x, if foreground { 6 } else { 5 });
            assert_eq!(first.tone, if foreground { 99.0 } else { 100.0 });
            let callbacks = trace
                .iter()
                .enumerate()
                .filter(|(_, sample)| sample.callback)
                .collect::<Vec<_>>();
            assert_eq!(callbacks.len(), 1, "{fps} FPS, foreground={foreground}");
            let (index, callback) = callbacks[0];
            for sample in &trace[..index] {
                assert_eq!(sample.hero, first.hero);
                assert_eq!(sample.npc, first.npc);
                assert_eq!(sample.tone, first.tone);
            }
            assert_eq!(callback.raw, trace[index - 1].raw);
            assert_eq!(callback.scene, 1);
            assert_eq!(
                callback.npc, first.npc,
                "the earlier NPC was already processed"
            );
            assert_eq!(callback.hero_x, 6);
            assert_eq!(callback.hero == first.hero, foreground);
            assert_eq!(callback.tone, 99.0);
            let fresh = &trace[index + 1];
            assert!(!fresh.callback);
            assert_eq!((fresh.raw, fresh.scene), (callback.raw + 1, 2));
            assert_ne!(fresh.npc, callback.npc);
            assert_ne!(fresh.hero, callback.hero);
            assert_eq!(fresh.tone, 98.0);
            assert!(switch_on(&app, 70));
        }
    }
}
