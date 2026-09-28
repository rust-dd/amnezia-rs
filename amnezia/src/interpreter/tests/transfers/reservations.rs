use super::*;
use crate::timing::GameFrames;
use crate::transitions::{Transition, TransitionPlugin};

mod clocks;

fn app() -> App {
    let mut app = interp_app();
    app.add_plugins((
        AssetPlugin::default(),
        TransitionPlugin,
        crate::teleport::TeleportPlugin,
    ))
    .init_asset::<Image>()
    .add_message::<crate::world::MapChanged>();
    let hero = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(hero)
        .insert(Transform::default());
    app.world_mut().resource_mut::<MapData>().map_id = 3;
    app.update();
    app
}

fn increment(id: i32) -> EventCommand {
    cmd(10220, 0, vec![0, id, id, 1, 0, 1])
}

fn counts(app: &App) -> [i32; 4] {
    let vars = app.world().resource::<Variables>();
    std::array::from_fn(|index| vars.get(index as u32 + 1))
}

fn tick(app: &mut App, frame: u32) {
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
}

#[test]
fn a_reserved_parallel_transfer_finishes_every_old_map_interpreter_before_erasure() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![
        common(1, 4, 0, vec![cmd(10810, 0, vec![3, 7, 8]), increment(1)]),
        common(2, 4, 0, vec![increment(2)]),
    ]));
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 4, vec![increment(3)])],
    });
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![increment(4)]);
    tick(&mut app, 0);
    assert_eq!(counts(&app), [1; 4]);
    assert!(app.world().resource::<Fade>().busy());
    assert!(app.world().resource::<Transition>().busy());
    assert_eq!(app.world().resource::<Transition>().age(), 0);
    tick(&mut app, 34);
    assert_eq!(counts(&app), [1; 4]);
}

#[test]
fn a_parallel_recall_runs_its_tail_in_the_same_visit() {
    for map in [3, 0] {
        let mut app = interp_app();
        {
            let mut vars = app.world_mut().resource_mut::<Variables>();
            vars.set(10, map);
            vars.set(11, 7);
            vars.set(12, 8);
        }
        app.insert_resource(CommonEvents(vec![common(
            1,
            4,
            0,
            vec![cmd(10830, 0, vec![10, 11, 12]), increment(1)],
        )]));
        app.update();
        assert_eq!(counts(&app), [1, 0, 0, 0]);
        assert_eq!(
            app.world().resource::<PendingTeleport>().0,
            (map > 0).then_some((map as u32, 7, 8))
        );
    }
}

#[test]
fn the_last_transfer_reserved_by_the_old_map_wins() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        4,
        0,
        vec![
            cmd(10810, 0, vec![3, 1, 2]),
            increment(1),
            cmd(11410, 0, vec![100]),
        ],
    )]));
    app.insert_resource(MapEvents {
        events: vec![map_event(
            1,
            4,
            vec![
                cmd(10810, 0, vec![3, 4, 5]),
                increment(2),
                cmd(11410, 0, vec![100]),
            ],
        )],
    });
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![cmd(10810, 0, vec![3, 7, 8]), increment(3)]);
    tick(&mut app, 0);
    assert_eq!(counts(&app), [1, 1, 0, 0]);
    tick(&mut app, 35);
    let world = app.world_mut();
    let hero = world.query::<&Player>().single(world).unwrap();
    assert_eq!((hero.tile_x, hero.tile_y), (7, 8));
    assert_eq!(counts(&app), [1, 1, 0, 0]);
}

#[test]
fn a_reserved_transfer_waits_for_the_whole_asynchronous_map_visit_to_resume() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![
        common(
            1,
            4,
            0,
            vec![
                cmd(10810, 0, vec![3, 7, 8]),
                cmd(11010, 0, vec![0]),
                increment(1),
                cmd(11410, 0, vec![100]),
            ],
        ),
        common(2, 4, 0, vec![increment(2)]),
    ]));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![increment(3)]);
    tick(&mut app, 0);
    assert!(!app.world().resource::<Fade>().busy());
    assert_eq!(counts(&app), [0; 4]);
    tick(&mut app, 35);
    assert_eq!(counts(&app), [1, 2, 1, 0]);
    assert!(!app.world().resource::<Fade>().busy());
    assert!(app.world().resource::<Transition>().erased());
    assert!(app.world().resource::<Transition>().event_erased);
    let world = app.world_mut();
    let hero = world.query::<&Player>().single(world).unwrap();
    assert_eq!((hero.tile_x, hero.tile_y), (7, 8));
    assert!(app.world().resource::<PendingTeleport>().0.is_none());
}

#[test]
fn a_later_scene_request_waits_for_the_reserved_transfer() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        4,
        0,
        vec![cmd(10810, 0, vec![3, 7, 8]), cmd(11410, 0, vec![100])],
    )]));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![cmd(11910, 0, vec![])]);
    for frame in [0, 35, 36, 69] {
        tick(&mut app, frame);
        assert!(!app.world().resource::<EventSaveRequest>().0);
        assert!(
            app.world()
                .resource::<crate::interpreter::scenes::Requests>()
                .pending()
        );
    }
    tick(&mut app, 70);
    assert!(app.world().resource::<EventSaveRequest>().0);
}

#[test]
fn foreground_recall_yields_but_a_reserved_parallel_target_does_not_block_it() {
    let mut app = interp_app();
    {
        let mut vars = app.world_mut().resource_mut::<Variables>();
        vars.set(10, 3);
        vars.set(11, 7);
        vars.set(12, 8);
    }
    app.insert_resource(CommonEvents(vec![common(
        1,
        4,
        0,
        vec![cmd(10810, 0, vec![3, 1, 2]), increment(1)],
    )]));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![cmd(10830, 0, vec![10, 11, 12]), increment(2)]);
    app.update();
    assert_eq!(counts(&app), [1, 0, 0, 0]);
    assert_eq!(app.world().resource::<PendingTeleport>().0, Some((3, 7, 8)));
    assert_eq!(app.world().resource::<RunningEvent>().frame.ip, 1);
}
