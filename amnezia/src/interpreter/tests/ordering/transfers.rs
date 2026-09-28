use super::*;
use crate::world::{Character, EventSprite, test_support};

#[test]
fn reserved_transfers_keep_the_remaining_npc_and_hero_updates_on_the_old_map() {
    let page = test_support::page(vec![test_support::command(1, 0)]);
    let mut app = test_support::app(vec![test_support::event(1, 1, vec![page])], false);
    app.add_plugins((
        crate::transitions::TransitionPlugin,
        crate::teleport::TeleportPlugin,
    ))
    .add_message::<crate::world::MapChanged>();
    app.world_mut().resource_mut::<MapData>().map_id = 3;
    app.insert_resource(CommonEvents(vec![common(
        1,
        4,
        0,
        vec![cmd(10810, 0, vec![3, 7, 8])],
    )]));
    queue_hero_step(app.world_mut(), crate::tiles::DIR_RIGHT, 6);
    let npc = test_support::entity(&mut app, 1);
    app.world_mut()
        .get_mut::<MoveQueue>(npc)
        .unwrap()
        .push_step(crate::world::RouteAction::Step {
            dx: 1,
            dy: 0,
            face: crate::tiles::DIR_RIGHT,
        });
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![cmd(10220, 0, vec![0, 1, 1, 0, 6, 10001, 1])]);
    app.update();
    assert_eq!(hero_x(&mut app), 6);
    assert_eq!(app.world().resource::<Variables>().get(1), 6);
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile(), (2, 1));
    assert!(app.world().resource::<Fade>().busy());
}
