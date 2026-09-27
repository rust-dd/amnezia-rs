use super::*;

#[test]
fn a_relocated_heros_zero_remaining_jump_survives_an_actual_save_file() {
    let (mut app, path, entity) = hero_app("hero-relocation");
    start_motion(app.world_mut(), true);
    {
        let world = app.world_mut();
        let (mut hero, mut queue) = world
            .query::<(&mut Player, &mut MoveQueue)>()
            .single_mut(world)
            .unwrap();
        queue.use_character_motion(2, hero.dir);
        queue.relocate(hero.tile());
        hero.set_tile(4, 6);
    }
    let expected = app.world().get::<MoveQueue>(entity).unwrap().snapshot();
    assert!(expected.valid());
    save_and_load(&mut app);
    let original = std::fs::read_to_string(&path).unwrap();
    assert!(original.contains(&format!(
        "format_version: {}",
        crate::save::SAVE_FORMAT_VERSION
    )));
    assert_eq!(
        app.world().get::<MoveQueue>(entity).unwrap().snapshot(),
        expected
    );
    let data = app.world().resource::<MapData>();
    let hero = app.world().get::<Player>(entity).unwrap();
    let point = Vec2::from(data.tile_center(4, 6));
    assert_eq!(hero.tile(), (4, 6));
    assert_eq!(
        app.world()
            .get::<Transform>(entity)
            .unwrap()
            .translation
            .truncate(),
        point + Vec2::Y * hero.y_offset()
    );
    app.world_mut().resource_scope(|world, data: Mut<MapData>| {
        let (mut hero, mut queue, mut route) = world
            .query::<(&mut Player, &mut MoveQueue, &mut RouteStepper)>()
            .single_mut(world)
            .unwrap();
        assert!(
            drive_route(
                &mut *hero,
                &mut queue,
                &mut route,
                (4, 6),
                |_, _, _, _, _| true
            )
            .effects
            .is_empty()
        );
        assert_eq!(queue.advance(&mut *hero, &data, 1.0 / 60.0), Some(point));
        assert!(!queue.busy());
        assert!(queue.advance(&mut *hero, &data, 1.0 / 60.0).is_none());
    });
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}
