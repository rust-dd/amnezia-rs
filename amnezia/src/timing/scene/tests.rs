use super::*;
use crate::timing::{GameFrames, TimingPlugin};
use crate::transitions::{Kind, Transition, TransitionPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

fn app(fps: u32) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TimingPlugin, TransitionPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / fps as f64,
        )));
    app.update();
    app
}

#[test]
fn only_scene_ticks_stop_through_the_final_async_frame_at_every_render_rate() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = app(fps);
        app.world_mut().resource_mut::<SceneFrames>().frame = u32::MAX - 2;
        app.world_mut().resource_mut::<Transition>().start_for(
            Kind::Fade,
            false,
            0,
            IVec2::new(160, 120),
            35,
        );
        let mut renders = 0;
        while app.world().resource::<Transition>().busy() {
            app.update();
            renders += 1;
            assert!(renders < fps, "transition must finish on the raw clock");
            assert_eq!(app.world().resource::<SceneFrames>().frame, u32::MAX - 2);
            assert!(app.world().resource::<SceneWait>().0);
        }
        let start = app.world().resource::<GameFrames>().frame;
        assert!(start >= 35);
        for _ in 0..fps {
            app.update();
            assert!(!app.world().resource::<SceneWait>().0);
            let elapsed = app.world().resource::<GameFrames>().frame - start;
            assert_eq!(
                app.world().resource::<SceneFrames>().frame,
                (u32::MAX - 2).wrapping_add(elapsed),
                "shared tick phase at {fps} FPS"
            );
        }
    }
}

#[test]
fn erased_screens_and_open_menus_do_not_pause_the_original_counter() {
    let mut app = app(144);
    app.world_mut().resource_mut::<Transition>().hold_black();
    app.insert_resource(crate::menu::MenuOpen(true))
        .insert_resource(crate::shop::ShopOpen(true))
        .insert_resource(crate::battle::BattleActive(true));
    for _ in 0..144 {
        app.update();
    }
    assert_eq!(app.world().resource::<SceneFrames>().frame, 60);
    assert_eq!(app.world().resource::<GameFrames>().frame, 60);
}

#[test]
fn map_rebuild_requests_hold_the_counter_without_losing_the_raw_fraction() {
    let mut app = app(144);
    app.update();
    app.update();
    assert_eq!(app.world().resource::<SceneFrames>().frame, 0);
    app.insert_resource(crate::teleport::PendingTeleport::default());
    app.world_mut()
        .resource_mut::<crate::teleport::PendingTeleport>()
        .reload(5, 0, 0);
    for _ in 0..7 {
        app.update();
        assert_eq!(app.world().resource::<SceneFrames>().frame, 0);
    }
    app.world_mut()
        .resource_mut::<crate::teleport::PendingTeleport>()
        .0 = None;
    let raw = app.world().resource::<GameFrames>().frame;
    app.update();
    let ticks = app.world().resource::<GameFrames>().frame - raw;
    assert_eq!(app.world().resource::<SceneFrames>().frame, ticks);
    assert_eq!(
        ticks, 1,
        "resume uses the input clock's existing partial tick"
    );
}

#[test]
fn a_requested_new_game_cannot_age_the_scene_before_rebuilding() {
    let mut app = app(60);
    app.init_resource::<crate::session::NewGameRequest>();
    app.world_mut()
        .resource_mut::<crate::session::NewGameRequest>()
        .requested = true;
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(app.world().resource::<SceneFrames>().frame, 0);
    assert_eq!(app.world().resource::<GameFrames>().frame, 60);
}

#[test]
fn gameover_preparation_is_not_a_playing_scene() {
    let mut app = app(60);
    app.init_resource::<crate::gameover::GameOverFlow>();
    app.insert_resource(crate::gameover::GameOverActive(true));
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(app.world().resource::<SceneFrames>().frame, 0);
    app.world_mut()
        .resource_mut::<crate::gameover::GameOverActive>()
        .0 = false;
    app.update();
    assert_eq!(app.world().resource::<SceneFrames>().frame, 1);
}
