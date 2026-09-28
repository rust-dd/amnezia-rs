use super::*;
use crate::timing::{
    SceneFrames, TimingPlugin,
    logical::{self, LogicalPlugin, Step},
};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Resource, Default)]
struct Samples(Vec<Sample>);

#[derive(Debug, PartialEq)]
struct Sample {
    raw: u32,
    scene: u32,
    callback: bool,
    keys: [bool; 3],
    values: [i32; 3],
    pre: usize,
    elapsed: Duration,
}

#[derive(Resource, Default)]
struct Pre(usize);

#[allow(clippy::too_many_arguments)]
fn record(
    raw: Res<GameFrames>,
    scene: Res<SceneFrames>,
    step: Res<Step>,
    keys: Res<ButtonInput<KeyCode>>,
    variables: Res<Variables>,
    pre: Res<Pre>,
    time: Res<Time>,
    mut samples: ResMut<Samples>,
) {
    samples.0.push(Sample {
        raw: raw.frame,
        scene: scene.frame,
        callback: step.callback,
        keys: [
            keys.pressed(KeyCode::Space),
            keys.pressed(KeyCode::Enter),
            keys.just_pressed(KeyCode::Enter),
        ],
        values: std::array::from_fn(|index| variables.get(index as u32 + 1)),
        pre: pre.0,
        elapsed: time.elapsed(),
    });
}

fn prepared(commands: Vec<EventCommand>) -> App {
    prepared_owner(commands, false)
}

fn prepared_owner(commands: Vec<EventCommand>, parallel: bool) -> App {
    let mut app = interp_app();
    app.add_plugins((TimingPlugin, LogicalPlugin, TransitionPlugin))
        .init_resource::<Samples>()
        .init_resource::<Pre>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .add_systems(Update, record.after(crate::interpreter::scenes::Commit));
    logical::pre(&mut app, || |mut pre: ResMut<Pre>| pre.0 += 1);
    app.update();
    if parallel {
        app.insert_resource(CommonEvents(vec![common(1, 4, 0, commands)]));
    } else {
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(7, commands);
    }
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.update();
    while app.world().resource::<Transition>().busy() {
        assert!(app.world().resource::<GameFrames>().frame < 100);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
    }
    assert_eq!(app.world().resource::<SceneFrames>().frame, 1);
    app.world_mut().resource_mut::<Samples>().0.clear();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.clear();
    keys.release(KeyCode::Space);
    keys.press(KeyCode::Enter);
    app
}

#[test]
fn callback_uses_previous_input_then_fresh_update_samples_the_new_input() {
    let mut app = prepared(vec![cmd(11010, 0, vec![0]), increment(1)]);
    let raw = app.world().resource::<GameFrames>().frame;
    let pre = app.world().resource::<Pre>().0;
    app.update();
    let samples = &app.world().resource::<Samples>().0;
    assert_eq!(samples.len(), 2);
    assert_eq!(
        (
            samples[0].raw,
            samples[0].scene,
            samples[0].callback,
            samples[0].pre
        ),
        (raw, 1, true, pre)
    );
    assert_eq!(samples[0].keys, [true, false, false]);
    assert_eq!(
        (
            samples[1].raw,
            samples[1].scene,
            samples[1].callback,
            samples[1].pre
        ),
        (raw + 1, 2, false, pre + 1)
    );
    assert_eq!(samples[1].keys, [false, true, true]);
    assert_eq!(samples[0].values, [1, 0, 0]);
    assert_eq!(samples[1].values, [1, 0, 0]);
    assert_eq!(
        samples[1].elapsed - samples[0].elapsed,
        Duration::from_secs_f64(1.0 / 60.0)
    );
}

#[test]
fn a_resuspending_callback_skips_player_update_without_losing_buffered_input() {
    let mut app = prepared(vec![
        cmd(11010, 0, vec![0]),
        cmd(11020, 0, vec![0]),
        increment(1),
    ]);
    let raw = app.world().resource::<GameFrames>().frame;
    let pre = app.world().resource::<Pre>().0;
    app.update();
    assert_eq!(app.world().resource::<GameFrames>().frame, raw);
    assert_eq!(app.world().resource::<Pre>().0, pre);
    let samples = &app.world().resource::<Samples>().0;
    assert_eq!(samples.len(), 1);
    assert!(samples[0].callback);
    assert_eq!(samples[0].keys, [true, false, false]);
    assert_eq!(samples[0].values, [0; 3]);
    let keys = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(!keys.pressed(KeyCode::Space));
    assert!(keys.just_pressed(KeyCode::Enter));
    let elapsed = samples[0].elapsed;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    let samples = &app.world().resource::<Samples>().0;
    assert_eq!(samples.len(), 2);
    assert_eq!(samples[1].raw, raw + 1);
    assert_eq!(samples[1].scene, 1);
    assert!(!samples[1].callback);
    assert_eq!(samples[1].keys, [false, true, true]);
    assert_eq!(
        samples[1].elapsed - elapsed,
        Duration::from_secs_f64(1.0 / 60.0)
    );
}

#[test]
fn callback_keeps_the_old_command_budget_until_the_fresh_visit() {
    for parallel in [false, true] {
        let mut commands = vec![increment(1); 9998];
        commands.extend([cmd(11010, 0, vec![0]), increment(2), increment(3)]);
        let mut app = prepared_owner(commands, parallel);
        app.update();
        let samples = &app.world().resource::<Samples>().0;
        assert_eq!(samples.len(), 2);
        assert!(samples[0].callback);
        assert_eq!(samples[0].values, [9998, 1, 0]);
        assert_eq!(samples[1].values, [9998, 1, 1]);
    }
}

#[test]
fn committing_a_scene_from_a_callback_preempts_the_fresh_player_update() {
    let mut app = prepared(vec![cmd(11010, 0, vec![0]), cmd(11910, 0, vec![])]);
    let raw = app.world().resource::<GameFrames>().frame;
    let pre = app.world().resource::<Pre>().0;
    let committed = app
        .world()
        .resource::<crate::interpreter::scenes::Requests>()
        .committed();
    app.update();
    assert_eq!(app.world().resource::<GameFrames>().frame, raw);
    assert_eq!(app.world().resource::<Pre>().0, pre);
    assert_eq!(
        app.world()
            .resource::<crate::interpreter::scenes::Requests>()
            .committed(),
        committed + 1
    );
    assert!(app.world().resource::<EventSaveRequest>().0);
    let samples = &app.world().resource::<Samples>().0;
    assert_eq!(samples.len(), 1);
    assert!(samples[0].callback);
}

#[test]
fn returning_to_title_from_a_callback_also_preempts_the_fresh_player_update() {
    let mut app = prepared(vec![cmd(11010, 0, vec![0]), cmd(12510, 0, vec![])]);
    let raw = app.world().resource::<GameFrames>().frame;
    let pre = app.world().resource::<Pre>().0;
    app.update();
    assert_eq!(app.world().resource::<GameFrames>().frame, raw);
    assert_eq!(app.world().resource::<Pre>().0, pre);
    assert!(app.world().resource::<TitleActive>().0);
    let samples = &app.world().resource::<Samples>().0;
    assert_eq!(samples.len(), 1);
    assert!(samples[0].callback);
}
