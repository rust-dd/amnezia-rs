use super::*;

#[test]
fn another_foreground_event_suppresses_contact_reset_but_not_continued_movement() {
    let mut app = chasing_app();
    app.insert_resource(crate::dialogue::MessageOptions {
        fixed: false,
        continue_events: true,
    });
    let world = app.world_mut();
    world.resource_mut::<RunningEvent>().start(
        2,
        vec![amnezia_data::EventCommand {
            code: 11410,
            indent: 0,
            string: String::new(),
            params: vec![100],
        }],
    );
    let entity = world
        .query_filtered::<Entity, With<EventSprite>>()
        .single(world)
        .unwrap();
    world.get_mut::<AutoMove>(entity).unwrap().move_type = 2;
    let mut route = RouteStepper::from_page(&amnezia_data::MoveRouteDef::default(), 4, 3);
    route.set_stop_count(84);
    route.set_stop_maximum(64);
    world.entity_mut(entity).insert(route);
    app.update();
    assert_eq!(event_tile(&app), (5, 7));
    assert!(app.world().get::<MoveQueue>(entity).unwrap().busy());
    assert_eq!(
        app.world()
            .get::<RouteStepper>(entity)
            .unwrap()
            .stop_count(),
        84
    );
    assert!(
        app.world()
            .resource::<RunningEvent>()
            .queued_ids()
            .is_empty()
    );
}
