use super::*;
use crate::timing::{GameFrames, SceneWait};
use crate::transitions::{Transition, TransitionPlugin};

#[test]
fn destination_preupdate_moves_npcs_but_does_not_repeat_the_hero_stage() {
    let mut app = app();
    app.add_plugins((TransitionPlugin, crate::teleport::TeleportPlugin))
        .add_message::<crate::world::MapChanged>();
    app.world_mut().resource_mut::<MapData>().map_id = 3;
    let npc = npcs::npc(&mut app, 1, 1, vec![], &[1, 8, 0, 0, 1]);
    app.world_mut()
        .entity_mut(npc)
        .insert(RouteStepper::from_move_event(&[1, 8, 0, 0, 1]).with_speed(6));
    let world = app.world_mut();
    let hero = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    world
        .entity_mut(hero)
        .insert(RouteStepper::from_move_event(&[10001, 8, 0, 0, 1, 1]).with_speed(6));
    world
        .resource_mut::<RunningEvent>()
        .start(7, vec![cmd(10810, 0, vec![3, 7, 8])]);
    app.update();
    assert_eq!(hero_x(&mut app), 6);
    let before = app.world().get::<Transform>(npc).unwrap().translation.x;
    assert!(app.world().resource::<Transition>().busy());
    app.insert_resource(SceneWait(true));
    app.world_mut().resource_mut::<GameFrames>().frame = 35;
    app.update();
    assert_eq!(hero_x(&mut app), 7);
    assert_eq!(
        app.world().get::<Transform>(npc).unwrap().translation.x,
        before + 8.0
    );
    assert!(!app.world().get::<MoveQueue>(npc).unwrap().busy());
    assert!(!crate::world::update::claim(app.world_mut(), hero));
    assert!(!crate::world::update::claim(app.world_mut(), npc));
    assert_eq!(app.world().resource::<Transition>().age(), 0);
}
