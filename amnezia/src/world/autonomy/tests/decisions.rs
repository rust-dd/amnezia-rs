use super::*;

fn fixture(kind: u32, direction: u32, count: u32, maximum: u32, seed: u32) -> (App, Entity) {
    let mut app = app_with_mover(kind, maximum);
    let world = app.world_mut();
    let entity = world
        .query_filtered::<Entity, With<EventSprite>>()
        .single(world)
        .unwrap();
    world.get_mut::<AutoMove>(entity).unwrap().rng = seed;
    let (mut character, mut route) = world
        .query::<(&mut EventSprite, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    route.set_direction(&mut *character, direction);
    route.set_stop_count(count);
    (app, entity)
}

fn wall(app: &mut App, x: usize, y: usize) {
    let mut data = app.world_mut().resource_mut::<MapData>();
    let index = y * data.width as usize + x;
    data.upper[index] = 10001;
    data.passages_up[1] = 0;
}

#[test]
fn random_walking_uses_original_relative_direction_weights() {
    for (seed, direction, tile) in [
        (10, DIR_UP, (2, 1)),
        (9, DIR_UP, (2, 1)),
        (8, DIR_UP, (2, 1)),
        (7, DIR_LEFT, (1, 2)),
        (6, DIR_LEFT, (1, 2)),
        (5, DIR_RIGHT, (3, 2)),
        (4, DIR_RIGHT, (3, 2)),
        (3, DIR_DOWN, (2, 3)),
    ] {
        let (mut app, entity) = fixture(1, DIR_UP, 64, 64, seed);
        app.update();
        assert_eq!(event_tile(&app), tile, "seed {seed}");
        assert_eq!(
            app.world().get::<EventSprite>(entity).unwrap().dir,
            direction
        );
        assert!(app.world().get::<MoveQueue>(entity).unwrap().busy());
    }
}

#[test]
fn random_idle_draws_replace_only_the_elapsed_count() {
    for (seed, count, rng) in [(2, 35, 134253570), (1, 44, 67634689)] {
        let (mut app, entity) = fixture(1, DIR_UP, 100, 64, seed);
        app.update();
        assert_eq!(event_tile(&app), (2, 2));
        assert!(!app.world().get::<MoveQueue>(entity).unwrap().busy());
        let route = app.world().get::<RouteStepper>(entity).unwrap();
        assert_eq!((route.stop_count(), route.stop_maximum()), (count, 64));
        assert_eq!(app.world().get::<AutoMove>(entity).unwrap().rng, rng);
        assert_eq!(app.world().get::<EventSprite>(entity).unwrap().dir, DIR_UP);
    }
}

#[test]
fn a_cycle_reverses_only_after_twenty_extra_stopped_updates() {
    for (count, tile, direction) in [
        (64, (2, 2), DIR_DOWN),
        (83, (2, 2), DIR_DOWN),
        (84, (2, 1), DIR_UP),
    ] {
        let (mut app, entity) = fixture(2, DIR_DOWN, count, 64, 1);
        wall(&mut app, 2, 3);
        app.update();
        assert_eq!(event_tile(&app), tile, "count {count}");
        assert_eq!(
            app.world().get::<EventSprite>(entity).unwrap().dir,
            direction
        );
    }
}

#[test]
fn blocked_walkers_keep_retrying_until_the_sixty_update_backoff() {
    for (kind, seed, failed_direction) in [(1, 7, DIR_LEFT), (2, 1, DIR_DOWN), (4, 8, DIR_RIGHT)] {
        for count in [123, 124] {
            let (mut app, entity) = fixture(kind, DIR_UP, count, 64, seed);
            for (x, y) in [(2, 1), (3, 2), (2, 3), (1, 2)] {
                wall(&mut app, x, y);
            }
            app.update();
            assert_eq!(event_tile(&app), (2, 2));
            let route = app.world().get::<RouteStepper>(entity).unwrap();
            assert_eq!(route.stop_count(), if count == 124 { 0 } else { count });
            let expected = if count == 124 {
                failed_direction
            } else {
                DIR_UP
            };
            assert_eq!(
                app.world().get::<EventSprite>(entity).unwrap().dir,
                expected
            );
            assert!(!app.world().get::<MoveQueue>(entity).unwrap().busy());
        }
    }
}

#[test]
fn blocked_seekers_do_not_try_the_other_axis() {
    for kind in [4, 5] {
        let (mut app, entity) = fixture(kind, DIR_UP, 64, 64, 8);
        let world = app.world_mut();
        world
            .query::<&mut Player>()
            .single_mut(world)
            .unwrap()
            .tile_y = 4;
        wall(&mut app, if kind == 4 { 3 } else { 1 }, 2);
        app.update();
        assert_eq!(event_tile(&app), (2, 2));
        assert!(!app.world().get::<MoveQueue>(entity).unwrap().busy());
        assert_eq!(app.world().get::<EventSprite>(entity).unwrap().dir, DIR_UP);
    }
}

#[test]
fn visible_seekers_can_keep_heading_or_draw_a_random_direction() {
    for (seed, tile) in [(10, (2, 1)), (9, (3, 2))] {
        let (mut app, _) = fixture(4, DIR_UP, 64, 64, seed);
        let world = app.world_mut();
        let mut hero = world.query::<&mut Player>().single_mut(world).unwrap();
        hero.tile_x = 2;
        hero.tile_y = 5;
        app.update();
        assert_eq!(event_tile(&app), tile);
    }
}

#[test]
fn seeker_visibility_uses_the_original_inclusive_two_tile_screen_margin() {
    for (x, y, visible) in [
        (-33, 128, false),
        (-32, 128, true),
        (352, 128, true),
        (353, 128, false),
        (128, -33, false),
        (128, -32, true),
        (128, 272, true),
        (128, 273, false),
    ] {
        let (mut app, _) = fixture(4, DIR_DOWN, 64, 64, 8);
        let point = Vec2::from(app.world().resource::<MapData>().tile_center(2, 2));
        let camera = Vec2::new(point.x + 160.0 - x as f32, point.y + y as f32 - 128.0);
        assert_eq!(
            app.world()
                .resource::<MapData>()
                .screen_position(point, camera),
            (x, y)
        );
        app.world_mut().spawn((
            crate::world::MainCamera,
            Transform::from_translation(camera.extend(0.0)),
        ));
        app.update();
        assert_eq!(
            event_tile(&app),
            if visible { (3, 2) } else { (2, 1) },
            "({x},{y})"
        );
    }
}

#[test]
fn cycle_movement_does_not_consume_random_numbers() {
    let (mut app, entity) = fixture(2, DIR_UP, 64, 64, 999);
    app.update();
    assert_eq!(event_tile(&app), (2, 1));
    assert_eq!(app.world().get::<AutoMove>(entity).unwrap().rng, 999);
}

#[test]
fn a_collision_resets_the_count_before_a_cycle_can_reverse_even_with_no_commands() {
    for empty in [false, true] {
        let mut app = chasing_app();
        let world = app.world_mut();
        if empty {
            world.resource_mut::<MapEvents>().events[0].pages[0]
                .commands
                .clear();
        }
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
        assert_eq!(event_tile(&app), (5, 6));
        assert!(!app.world().get::<MoveQueue>(entity).unwrap().busy());
        assert_eq!(
            app.world()
                .get::<RouteStepper>(entity)
                .unwrap()
                .stop_count(),
            0
        );
        assert_eq!(
            app.world().resource::<RunningEvent>().queued_ids(),
            if empty { vec![] } else { vec![1] }
        );
    }
}
