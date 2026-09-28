use super::*;

fn prepared() -> App {
    let mut app = unstarted_app();
    crate::dialogue::testing::register_position(&mut app);
    app.insert_resource(MapData::for_test(20, 15));
    let world = app.world_mut();
    world.spawn((crate::world::MainCamera, Transform::default()));
    world.spawn((crate::dialogue::DialoguePanel, Node::default()));
    let (mut hero, mut transform) = world
        .query::<(&mut Player, &mut Transform)>()
        .single_mut(world)
        .unwrap();
    hero.tile_x = 10;
    hero.tile_y = 9;
    transform.translation = Vec3::new(8.0, -32.0, 4.0);
    app.update();
    app
}

fn message() -> EventCommand {
    EventCommand {
        string: "Opening position".into(),
        ..cmd(10110, 0, vec![])
    }
}

#[test]
fn message_placement_uses_the_hero_at_the_command_not_after_movement() {
    let mut app = prepared();
    app.world_mut()
        .resource_mut::<crate::dialogue::MessageOptions>()
        .continue_events = true;
    app.insert_resource(CommonEvents(vec![
        common(1, 4, 0, vec![message()]),
        common(2, 4, 0, vec![cmd(11330, 0, vec![10001, 8, 0, 0, 0])]),
    ]));
    app.update();
    let world = app.world_mut();
    assert_eq!(world.query::<&Player>().single(world).unwrap().tile_y, 8);
    crate::dialogue::verify_placement(world, true);
}

#[test]
fn number_prompt_placement_is_fixed_before_later_forced_movement() {
    let mut app = prepared();
    app.world_mut()
        .resource_mut::<crate::dialogue::MessageOptions>()
        .continue_events = true;
    app.insert_resource(CommonEvents(vec![
        common(1, 4, 0, vec![cmd(10150, 0, vec![3, 10])]),
        common(2, 4, 0, vec![cmd(11330, 0, vec![10001, 8, 0, 0, 0])]),
    ]));
    app.update();
    let world = app.world_mut();
    assert!(world.resource::<InputNumber>().active());
    assert_eq!(world.query::<&Player>().single(world).unwrap().tile_y, 8);
    crate::dialogue::verify_placement(world, true);
}
