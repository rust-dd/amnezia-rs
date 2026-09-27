use super::*;

#[test]
fn a_relocated_npcs_zero_remaining_jump_survives_an_actual_save_file() {
    let (mut app, path) = app("npc-relocation");
    let entity = npc(app.world_mut());
    app.world_mut().resource_scope(|world, data: Mut<MapData>| {
        let (mut ch, mut queue) = world
            .query::<(&mut EventSprite, &mut MoveQueue)>()
            .get_mut(world, entity)
            .unwrap();
        queue.use_character_motion(2, tiles::DIR_RIGHT);
        queue.begin_from(
            &mut *ch,
            &data,
            (2, 3),
            RouteAction::Jump {
                dx: 2,
                dy: 1,
                face: tiles::DIR_RIGHT,
            },
        );
        queue.relocate(ch.tile());
        ch.set_tile(4, 6);
    });
    let expected = app.world().get::<MoveQueue>(entity).unwrap().snapshot();
    assert!(expected.valid_for_event());
    save_and_load(&mut app);
    let entity = npc(app.world_mut());
    let queue = app.world().get::<MoveQueue>(entity).unwrap();
    assert_eq!(queue.snapshot(), expected);
    let data = app.world().resource::<MapData>();
    let ch = app.world().get::<EventSprite>(entity).unwrap();
    let point = Vec2::from(data.tile_center(4, 6));
    assert_eq!(ch.tile(), (4, 6));
    assert!(queue.jumping());
    assert_eq!(queue.render_position(ch, data), point);
    std::fs::remove_file(path).unwrap();
}
