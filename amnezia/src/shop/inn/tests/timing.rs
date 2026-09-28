use super::*;
use crate::timing::{SceneFrames, SceneWait, TimingPlugin};

#[test]
fn inn_closing_and_music_count_scene_time_but_actual_transitions_do_not() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TimingPlugin))
            .init_resource::<State>()
            .init_resource::<Transition>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs_f64(1.0 / fps as f64),
            ));
        app.update();
        let mut expected_scene = 0;
        for (phase, paused) in [
            (Phase::Prompt { cost: 30 }, false),
            (Phase::Closing, false),
            (
                Phase::FadeOut {
                    started: Duration::ZERO,
                },
                true,
            ),
            (
                Phase::Resting {
                    started: Duration::ZERO,
                },
                false,
            ),
            (Phase::FadeIn, true),
            (Phase::Idle, false),
        ] {
            app.world_mut().resource_mut::<State>().phase = phase;
            let start = app.world().resource::<GameFrames>().frame;
            app.world_mut().resource_mut::<Transition>().clear();
            if paused {
                app.world_mut().resource_mut::<Transition>().start(
                    crate::transitions::Kind::Fade,
                    true,
                    start,
                    IVec2::ZERO,
                );
            }
            for _ in 0..fps {
                app.update();
                assert_eq!(app.world().resource::<SceneWait>().0, paused);
                let delta = app
                    .world()
                    .resource::<GameFrames>()
                    .frame
                    .wrapping_sub(start);
                assert_eq!(
                    app.world().resource::<SceneFrames>().frame,
                    expected_scene + if paused { 0 } else { delta },
                );
            }
            assert_eq!(app.world().resource::<GameFrames>().frame - start, 60);
            expected_scene += if paused { 0 } else { 60 };
        }
        assert_eq!(expected_scene, 240);
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
