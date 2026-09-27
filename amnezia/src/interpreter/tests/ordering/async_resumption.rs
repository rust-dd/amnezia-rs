use super::*;
use crate::timing::{GameFrames, SceneWait};
use crate::transitions::{Transition, TransitionPlugin};

fn tick(app: &mut App, frame: u32) {
    let waiting = app.world().resource::<Transition>().busy();
    app.insert_resource(SceneWait(waiting));
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
}

#[test]
fn an_interrupted_map_visit_keeps_processed_npcs_and_resumes_the_unprocessed_hero() {
    let mut app = app();
    app.add_plugins(TransitionPlugin);
    let first = npcs::npc(&mut app, 1, 1, vec![], &[1, 8, 0, 0, 1]);
    npcs::npc(
        &mut app,
        2,
        3,
        vec![cmd(11010, 0, vec![0]), cmd(11410, 0, vec![100])],
        &[2, 8, 0, 0, 3],
    );
    queue_hero_step(app.world_mut(), crate::tiles::DIR_RIGHT, 4);
    tick(&mut app, 0);
    assert_eq!(hero_x(&mut app), 5);
    let before = *app.world().get::<Transform>(first).unwrap();
    for frame in 1..35 {
        tick(&mut app, frame);
        assert_eq!(hero_x(&mut app), 5);
        assert_eq!(*app.world().get::<Transform>(first).unwrap(), before);
    }
    tick(&mut app, 35);
    assert_eq!(hero_x(&mut app), 6);
    assert_eq!(*app.world().get::<Transform>(first).unwrap(), before);
    tick(&mut app, 36);
    assert_ne!(*app.world().get::<Transform>(first).unwrap(), before);
}

#[test]
fn a_foreground_resume_does_not_advance_completed_character_movement_again() {
    let mut app = app();
    app.add_plugins(TransitionPlugin);
    let first = npcs::npc(&mut app, 1, 1, vec![], &[1, 8, 0, 0, 1]);
    queue_hero_step(app.world_mut(), crate::tiles::DIR_RIGHT, 4);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![cmd(11010, 0, vec![0]), switch_cmd(70, 0, 0)]);
    tick(&mut app, 0);
    let world = app.world_mut();
    let hero = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    let before = [
        *world.get::<Transform>(first).unwrap(),
        *world.get::<Transform>(hero).unwrap(),
    ];
    for frame in 1..=35 {
        tick(&mut app, frame);
        assert_eq!(*app.world().get::<Transform>(first).unwrap(), before[0]);
        assert_eq!(*app.world().get::<Transform>(hero).unwrap(), before[1]);
    }
    assert!(switch_on(&app, 70));
    tick(&mut app, 36);
    assert_ne!(*app.world().get::<Transform>(first).unwrap(), before[0]);
    assert_ne!(*app.world().get::<Transform>(hero).unwrap(), before[1]);
}
