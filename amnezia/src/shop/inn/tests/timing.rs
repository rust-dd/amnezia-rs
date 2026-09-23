use super::*;
use crate::timing::{SceneFrames, SceneWait, TimingPlugin};

#[test]
fn inn_prompts_count_scene_time_but_overnight_handoffs_only_count_raw_time() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TimingPlugin))
            .init_resource::<State>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs_f64(1.0 / fps as f64),
            ));
        app.update();
        app.world_mut().resource_mut::<State>().phase = Phase::Prompt { cost: 30 };
        for _ in 0..fps {
            app.update();
        }
        assert_eq!(app.world().resource::<SceneFrames>().frame, 60);
        for phase in [
            Phase::Closing,
            Phase::FadeOut {
                started: Duration::ZERO,
            },
            Phase::Resting {
                started: Duration::ZERO,
            },
            Phase::FadeIn,
        ] {
            app.world_mut().resource_mut::<State>().phase = phase;
            let start = app.world().resource::<GameFrames>().frame;
            for _ in 0..fps {
                app.update();
                assert_eq!(app.world().resource::<SceneFrames>().frame, 60);
                assert!(app.world().resource::<SceneWait>().0);
            }
            assert_eq!(app.world().resource::<GameFrames>().frame - start, 60);
        }
        app.world_mut().resource_mut::<State>().phase = Phase::Idle;
        for _ in 0..fps {
            app.update();
        }
        assert_eq!(app.world().resource::<SceneFrames>().frame, 120);
    }
}

#[test]
fn inn_question_behaves_like_an_ordinary_message_without_freezing_the_scene() {
    let mut app = app();
    open(&mut app, 30);
    let check = |scene: crate::world::ScenePause, pause: crate::dialogue::MessagePause| {
        (scene.paused(), pause.paused())
    };
    assert_eq!(
        app.world_mut().run_system_once(check).unwrap(),
        (false, false)
    );
    app.insert_resource(crate::menu::MenuOpen(true));
    assert_eq!(
        app.world_mut().run_system_once(check).unwrap(),
        (true, true)
    );
}
