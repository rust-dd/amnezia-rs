use super::*;
use crate::timing::{FrameClockSet, SceneFrames, SceneWait, TimingPlugin};
use bevy::time::TimeUpdateStrategy;

mod repetition;

#[derive(Debug, PartialEq)]
struct Sample {
    stage: u8,
    frame: u32,
    scene: u32,
    delta: Duration,
    held: bool,
    pressed: bool,
    released: bool,
}

#[derive(Resource, Default)]
struct Trace {
    samples: Vec<Sample>,
    renders: usize,
}

fn sample<const STAGE: u8>(
    frames: Res<GameFrames>,
    scene: Res<SceneFrames>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut trace: ResMut<Trace>,
) {
    trace.samples.push(Sample {
        stage: STAGE,
        frame: frames.frame,
        scene: scene.frame,
        delta: time.delta(),
        held: keys.pressed(KeyCode::Enter),
        pressed: keys.just_pressed(KeyCode::Enter),
        released: keys.just_released(KeyCode::Enter),
    });
}

fn duration(app: &mut App, seconds: f64) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        seconds,
    )));
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TimingPlugin, LogicalPlugin))
        .init_resource::<Trace>()
        .add_systems(Update, sample::<1>)
        .add_systems(PostUpdate, |mut trace: ResMut<Trace>| trace.renders += 1);
    pre(&mut app, || sample::<0>.after(FrameClockSet));
    post(&mut app, || sample::<2>);
    duration(&mut app, 0.0);
    app.update();
    assert_eq!(app.world().resource::<Trace>().samples.len(), 3);
    assert_eq!(app.world().resource::<GameFrames>().frame, 0);
    *app.world_mut().resource_mut::<Trace>() = Trace::default();
    app
}

#[test]
fn every_render_rate_runs_sixty_complete_ordered_game_updates() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = app();
        duration(&mut app, 1.0 / f64::from(fps));
        for _ in 0..fps {
            app.update();
            assert_eq!(
                app.world().resource::<Time>().delta(),
                Duration::from_secs_f64(1.0 / f64::from(fps)),
                "render time must be restored"
            );
        }
        let trace = app.world().resource::<Trace>();
        assert_eq!(trace.renders, fps as usize);
        assert_eq!(trace.samples.len(), 180, "{fps} FPS");
        for (index, sample) in trace.samples.iter().enumerate() {
            assert_eq!(sample.stage, (index % 3) as u8);
            assert_eq!(sample.frame, (index / 3 + 1) as u32);
            assert_eq!(sample.scene, sample.frame);
            assert_eq!(sample.delta, Duration::from_secs_f64(1.0 / 60.0));
        }
    }
}

#[test]
fn zero_tick_renders_preserve_a_tap_and_do_not_replay_it_during_catch_up() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        keys.release(KeyCode::Enter);
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    for _ in 0..80 {
        app.update();
    }
    assert!(app.world().resource::<Trace>().samples.is_empty());
    duration(&mut app, 1.0 / 15.0);
    app.update();
    let trace = app.world().resource::<Trace>();
    assert_eq!(trace.samples.len(), 12);
    for sample in &trace.samples {
        assert!(!sample.held);
        assert_eq!(sample.pressed, sample.frame == 1);
        assert_eq!(sample.released, sample.frame == 1);
    }
    let raw = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(!raw.just_pressed(KeyCode::Enter));
    assert!(!raw.just_released(KeyCode::Enter));
}

#[test]
fn holding_input_survives_catch_up_but_its_edge_is_delivered_once() {
    let mut app = app();
    duration(&mut app, 1.0 / 15.0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    for sample in &app.world().resource::<Trace>().samples {
        assert!(sample.held);
        assert_eq!(sample.pressed, sample.frame == 1);
        assert!(!sample.released);
    }
    let raw = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(raw.pressed(KeyCode::Enter));
    assert!(!raw.just_pressed(KeyCode::Enter));
}

#[test]
fn consumed_triggers_stay_consumed_after_returning_to_the_render_schedule() {
    fn discard_trigger(mut keys: ResMut<ButtonInput<KeyCode>>) {
        keys.clear_just_pressed(KeyCode::Enter);
    }
    let mut app = app();
    app.add_systems(Update, discard_trigger.before(sample::<1>));
    duration(&mut app, 1.0 / 60.0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    let keys = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(keys.pressed(KeyCode::Enter));
    assert!(!keys.just_pressed(KeyCode::Enter));
}

#[test]
fn transitions_release_the_scene_on_the_next_tick_within_the_same_render() {
    let mut app = app();
    app.add_plugins(crate::transitions::TransitionPlugin);
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .start_for(
            crate::transitions::Kind::Fade,
            false,
            0,
            IVec2::new(160, 120),
            3,
        );
    duration(&mut app, 1.0 / 10.0);
    app.update();
    for sample in &app.world().resource::<Trace>().samples {
        assert_eq!(sample.scene, sample.frame.saturating_sub(4));
    }
    assert_eq!(app.world().resource::<GameFrames>().frame, 6);
    assert!(!app.world().resource::<SceneWait>().0);
}

#[test]
fn restored_clock_and_fraction_are_not_overwritten_by_the_render_budget() {
    #[derive(Resource)]
    struct Restore(bool);
    fn restore(mut request: ResMut<Restore>, mut frames: ResMut<GameFrames>) {
        if std::mem::take(&mut request.0) {
            frames.frame = 100;
            frames.fraction = 0.25;
        }
    }
    let mut app = app();
    app.insert_resource(Restore(true));
    pre(&mut app, || restore.before(FrameClockSet));
    duration(&mut app, 1.0 / 15.0);
    app.update();
    assert_eq!(app.world().resource::<GameFrames>().frame, 104);
    assert_eq!(app.world().resource::<GameFrames>().fraction, 0.25);
    duration(&mut app, 0.0);
    app.update();
    assert_eq!(app.world().resource::<GameFrames>().frame, 104);
    duration(&mut app, 1.0 / 80.0);
    app.update();
    assert_eq!(app.world().resource::<GameFrames>().frame, 105);
    assert_eq!(app.world().resource::<GameFrames>().fraction, 0.0);
}

#[test]
fn both_frame_counters_wrap_without_losing_updates() {
    let mut app = app();
    app.world_mut().resource_mut::<GameFrames>().frame = u32::MAX - 1;
    app.world_mut().resource_mut::<SceneFrames>().frame = u32::MAX - 1;
    duration(&mut app, 1.0 / 15.0);
    app.update();
    assert_eq!(app.world().resource::<GameFrames>().frame, 2);
    assert_eq!(app.world().resource::<SceneFrames>().frame, 2);
    assert_eq!(app.world().resource::<Trace>().samples.len(), 12);
}
