use super::*;

#[test]
fn an_npcs_diagonal_retry_pose_and_locked_movement_survive_a_real_save_file() {
    for direction in 4..8 {
        let (mut app, path) = app(&format!("diagonal-retry-{direction}"));
        {
            let world = app.world_mut();
            let (mut character, mut route, mut queue) = world
                .query::<(&mut EventSprite, &mut RouteStepper, &mut MoveQueue)>()
                .single_mut(world)
                .unwrap();
            character.charset = "Chara1".into();
            route.restore_retry_direction(&mut *character, direction);
            assert_eq!(character.dir, direction);
            route.force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 26, 1]));
            drive_route(
                &mut *character,
                &mut queue,
                &mut route,
                (0, 0),
                |_, _, _, _, _| true,
            );
            assert!(queue.busy());
            assert_eq!(character.dir, direction);
        }
        let before = snapshot(app.world_mut());
        save_and_load(&mut app);
        assert_eq!(snapshot(app.world_mut()), before);
        let original = std::fs::read(&path).unwrap();
        let entity = npc(app.world_mut());
        assert_eq!(
            *app.world().get::<Visibility>(entity).unwrap(),
            Visibility::Hidden
        );
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert_eq!(snapshot(app.world_mut()), before);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}
