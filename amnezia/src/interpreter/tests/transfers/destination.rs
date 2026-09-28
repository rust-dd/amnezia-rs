use super::*;
use crate::timing::{GameFrames, SceneWait};
use crate::transitions::{Transition, TransitionPlugin};

fn app() -> App {
    let mut app = interp_app();
    app.add_plugins((
        AssetPlugin::default(),
        TransitionPlugin,
        crate::teleport::TeleportPlugin,
    ))
    .init_asset::<Image>()
    .add_message::<crate::world::MapChanged>()
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    let world = app.world_mut();
    let hero = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    world.entity_mut(hero).insert(Transform::default());
    world.resource_mut::<MapData>().map_id = 3;
    app.update();
    app
}

fn tick(app: &mut App, frame: u32) {
    let waiting = app.world().resource::<Transition>().busy()
        || app.world().resource::<Fade>().busy()
        || app.world().resource::<PendingTeleport>().reloading();
    app.insert_resource(SceneWait(waiting));
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
}

fn hero_x(app: &mut App) -> i32 {
    let world = app.world_mut();
    world.query::<&Player>().single(world).unwrap().tile_x
}

fn increment(id: i32, indent: u32) -> EventCommand {
    cmd(10220, indent, vec![0, id, id, 1, 0, 1])
}

fn when_arriving(mut body: Vec<EventCommand>) -> Vec<EventCommand> {
    let mut commands = vec![
        cmd(10220, 0, vec![0, 10, 10, 0, 6, 10001, 1]),
        cmd(12010, 0, vec![1, 10, 0, 7, 0]),
    ];
    for command in &mut body {
        command.indent = 1;
    }
    commands.extend(body);
    commands.push(cmd(22011, 0, vec![]));
    commands
}

fn start_foreground_transfer(app: &mut App) {
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![cmd(10810, 0, vec![3, 7, 8]), increment(3, 0)]);
}

fn reach_destination(app: &mut App) {
    for frame in 0..90 {
        tick(app, frame);
        if hero_x(app) != 5 {
            return;
        }
    }
    panic!("the transfer never reached its destination");
}

#[test]
fn destination_common_and_map_events_run_before_the_show_transition() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![common(1, 4, 0, vec![increment(1, 0)])]));
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 4, vec![increment(2, 0)])],
    });
    start_foreground_transfer(&mut app);
    reach_destination(&mut app);
    let vars = app.world().resource::<Variables>();
    assert_eq!([vars.get(1), vars.get(2), vars.get(3)], [2, 2, 0]);
    assert!(app.world().resource::<Fade>().busy());
    assert!(app.world().resource::<Transition>().busy());
    assert_eq!(app.world().resource::<Transition>().age(), 0);
}

#[test]
fn recursive_destination_transfers_finish_before_any_intermediate_map_is_shown() {
    let mut app = app();
    let mut commands = when_arriving(vec![cmd(10810, 0, vec![3, 9, 8]), increment(1, 0)]);
    commands.push(increment(2, 0));
    app.insert_resource(CommonEvents(vec![common(1, 4, 0, commands)]));
    start_foreground_transfer(&mut app);
    reach_destination(&mut app);
    assert_eq!(hero_x(&mut app), 9);
    let vars = app.world().resource::<Variables>();
    assert_eq!([vars.get(1), vars.get(2), vars.get(3)], [1, 3, 0]);
    assert!(app.world().resource::<PendingTeleport>().0.is_none());
    assert!(app.world().resource::<Transition>().busy());
    assert_eq!(app.world().resource::<Transition>().age(), 0);
}

#[test]
fn reloads_preupdate_parallel_events_before_the_initial_show() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        4,
        0,
        vec![increment(4700, 0)],
    )]));
    app.world_mut()
        .resource_mut::<PendingTeleport>()
        .reload(3, 7, 8);
    reach_destination(&mut app);
    assert_eq!(app.world().resource::<Variables>().get(4700), 1);
    assert!(app.world().resource::<Fade>().busy());
    assert!(app.world().resource::<Transition>().busy());
}

#[test]
fn an_erase_during_destination_preupdate_does_not_make_the_screen_stay_black() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        4,
        0,
        when_arriving(vec![cmd(11010, 0, vec![17]), increment(1, 0)]),
    )]));
    start_foreground_transfer(&mut app);
    reach_destination(&mut app);
    assert_eq!(app.world().resource::<Variables>().get(1), 1);
    assert!(!app.world().resource::<Transition>().event_erased);
    assert!(app.world().resource::<Transition>().busy());
}

#[test]
fn only_the_outer_foreground_transfer_runs_a_foreground_preupdate_after_show() {
    for foreground in [false, true] {
        for recursive in [false, true] {
            let mut app = app();
            set_switch(&mut app, 100, !foreground);
            let mut arrival = vec![switch_cmd(5, 0, 0)];
            if recursive {
                arrival.push(cmd(10810, 0, vec![3, 9, 8]));
            }
            let mut observer = when_arriving(arrival);
            observer.push(increment(1, 0));
            app.insert_resource(CommonEvents(vec![
                common(
                    1,
                    4,
                    100,
                    vec![cmd(10810, 0, vec![3, 7, 8]), switch_cmd(100, 1, 0)],
                ),
                common(2, 4, 0, observer),
                common(3, 3, 5, vec![increment(4, 0), switch_cmd(5, 1, 0)]),
            ]));
            if foreground {
                start_foreground_transfer(&mut app);
            }
            let mut transferring = false;
            let mut finished = false;
            for frame in 0..120 {
                tick(&mut app, frame);
                if app.world().resource::<Fade>().busy() {
                    transferring = true;
                } else if transferring {
                    finished = true;
                    break;
                }
            }
            assert!(finished);
            let vars = app.world().resource::<Variables>();
            assert_eq!(vars.get(1), 2 + i32::from(recursive));
            assert_eq!(vars.get(3), i32::from(foreground));
            assert_eq!(
                vars.get(4),
                i32::from(foreground),
                "foreground={foreground}, recursive={recursive}"
            );
        }
    }
}

#[test]
fn an_async_destination_visit_resumes_at_its_common_or_map_owner() {
    for map_owner in [false, true] {
        let mut app = app();
        let commands = when_arriving(vec![
            increment(1, 0),
            cmd(11020, 0, vec![19]),
            increment(2, 0),
            cmd(11410, 0, vec![100]),
        ]);
        let observer = increment(4, 0);
        if map_owner {
            app.insert_resource(MapEvents {
                events: vec![map_event(1, 4, commands), map_event(2, 4, vec![observer])],
            });
        } else {
            app.insert_resource(CommonEvents(vec![
                common(1, 4, 0, commands),
                common(2, 4, 0, vec![observer]),
            ]));
        }
        start_foreground_transfer(&mut app);
        reach_destination(&mut app);
        let vars = app.world().resource::<Variables>();
        assert_eq!([vars.get(1), vars.get(2), vars.get(4)], [1, 0, 1]);
        assert!(
            app.world()
                .resource::<super::super::super::continuation::Continuation>()
                .waiting()
        );
        let now = app.world().resource::<GameFrames>().frame;
        tick(&mut app, now + 1);
        let vars = app.world().resource::<Variables>();
        assert_eq!(
            [vars.get(1), vars.get(2), vars.get(3), vars.get(4)],
            [1, 1, 0, 2]
        );
        assert!(app.world().resource::<Fade>().busy());
        assert_eq!(app.world().resource::<Transition>().age(), 0);
    }
}

#[test]
fn a_suspended_destination_owner_retains_its_command_budget() {
    let mut app = app();
    let mut commands = vec![increment(1, 0); 9_996];
    commands.extend([cmd(11020, 0, vec![19]), increment(2, 0), increment(4, 0)]);
    app.insert_resource(CommonEvents(vec![common(1, 4, 0, when_arriving(commands))]));
    start_foreground_transfer(&mut app);
    reach_destination(&mut app);
    assert_eq!(app.world().resource::<Variables>().get(1), 9_996);
    let now = app.world().resource::<GameFrames>().frame;
    tick(&mut app, now + 1);
    let vars = app.world().resource::<Variables>();
    assert_eq!([vars.get(2), vars.get(4)], [1, 0]);
    assert!(app.world().resource::<Fade>().busy());
}

#[test]
fn an_async_foreground_preupdate_resumes_before_the_transfer_is_released() {
    let mut app = app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        7,
        vec![
            cmd(10810, 0, vec![3, 7, 8]),
            increment(1, 0),
            cmd(11020, 0, vec![19]),
            increment(2, 0),
        ],
    );
    reach_destination(&mut app);
    let now = app.world().resource::<GameFrames>().frame;
    tick(&mut app, now + 35);
    let vars = app.world().resource::<Variables>();
    assert_eq!([vars.get(1), vars.get(2)], [1, 0]);
    assert!(app.world().resource::<Fade>().busy());
    tick(&mut app, now + 36);
    let vars = app.world().resource::<Variables>();
    assert_eq!([vars.get(1), vars.get(2)], [1, 1]);
    assert!(!app.world().resource::<Fade>().busy());
}

#[test]
fn returning_to_title_during_preupdate_does_not_show_the_destination() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        4,
        0,
        when_arriving(vec![cmd(12510, 0, vec![])]),
    )]));
    start_foreground_transfer(&mut app);
    reach_destination(&mut app);
    assert!(app.world().resource::<TitleActive>().0);
    assert!(!app.world().resource::<Fade>().busy());
    assert!(!app.world().resource::<Transition>().busy());
    assert!(app.world().resource::<Transition>().erased());
}
