use super::*;
use crate::appearance::{ActorGraphics, Appearance, AppearancePlugin};
use crate::player::Player;

mod continuation;
mod diagonals;
mod lifecycle;
mod relocation;

fn hero_app(tag: &str) -> (App, std::path::PathBuf, Entity) {
    let (mut app, path) = app(tag);
    app.add_plugins((crate::gamedata::GameDataPlugin, AppearancePlugin));
    app.add_systems(
        Update,
        crate::player::update_player_sprite.after(ActorGraphics),
    );
    let world = app.world_mut();
    let entity = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    world.entity_mut(entity).insert((
        MoveQueue::default(),
        RouteStepper::default(),
        Sprite::default(),
        Transform::default(),
    ));
    app.update();
    (app, path, entity)
}

fn start_motion(world: &mut World, jumping: bool) {
    world.resource_scope(|world, data: Mut<MapData>| {
        let (mut hero, mut queue, mut route) = world
            .query_filtered::<(&mut Player, &mut MoveQueue, &mut RouteStepper), Without<EventSprite>>()
            .single_mut(world).unwrap();
        let mut program = vec![10001, 8, 0, 0, 36, 40, 26];
        program.extend(if jumping { vec![24, 1, 1, 2, 25] } else { vec![1] });
        program.extend([23, 27, 37, 1]);
        route.force_route(RouteStepper::from_move_event(&program));
        drive_route(&mut *hero, &mut queue, &mut route, (10, 10), |_, _, _, _, _| true);
        queue.advance(&mut *hero, &data, 0.04);
        route.animation.paused = true;
        hero.frame = 2;
        assert!(queue.busy() && route.pending());
    });
}

#[test]
fn saved_hero_steps_jumps_and_route_state_survive_the_rebuilt_map() {
    for jumping in [false, true] {
        let (mut app, path, entity) = hero_app(&format!("hero-motion-{jumping}"));
        start_motion(app.world_mut(), jumping);
        let expected = app.world().get::<MoveQueue>(entity).unwrap().snapshot();
        let mut expected_route = app.world().get::<RouteStepper>(entity).unwrap().clone();
        expected_route.reset_transparency();
        save_and_load(&mut app);
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            app.world().get::<MoveQueue>(entity).unwrap().snapshot(),
            expected
        );
        assert_eq!(
            app.world().get::<RouteStepper>(entity).unwrap(),
            &expected_route
        );
        let hero = app.world().get::<Player>(entity).unwrap();
        assert_eq!(hero.frame, 2);
        let position = expected
            .into_queue()
            .render_position(hero, app.world().resource::<MapData>());
        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().translation,
            Vec3::new(
                position.x,
                position.y + hero.y_offset(),
                hero.draw_z(hero.tile_y)
            )
        );
    }
}

#[test]
fn a_saved_route_resumes_but_its_temporary_graphic_and_opacity_do_not() {
    let (mut app, path, entity) = hero_app("hero-costume");
    app.world_mut()
        .resource_mut::<Appearance>()
        .set(1, "Chara4".into(), 3);
    app.update();
    start_motion(app.world_mut(), false);
    app.world_mut()
        .get_mut::<Player>(entity)
        .unwrap()
        .set_graphic("Poses2".into(), 4);
    save_and_load(&mut app);
    std::fs::remove_file(path).unwrap();
    let hero = app.world().get::<Player>(entity).unwrap();
    assert_eq!((hero.charset.as_str(), hero.index), ("Chara4", 3));
    assert!(app.world().get::<MoveQueue>(entity).unwrap().busy());
    assert_eq!(
        app.world().get::<RouteStepper>(entity).unwrap().alpha(),
        1.0
    );
    let source = app
        .world()
        .resource::<AssetServer>()
        .load::<Image>(resolve_png("CharSet", "Chara4"));
    let sprite = app.world().get::<Sprite>(entity).unwrap();
    assert_eq!(sprite.image, source);
    let (x, y) = tiles::charset_source(3, hero.dir, 2);
    assert_eq!(sprite.rect, Some(Rect::new(x, y, x + 24.0, y + 32.0)));
}

#[test]
fn a_mid_step_hero_repaints_changed_graphics_without_movement_ticks() {
    let (mut app, _, entity) = hero_app("hero-paused-graphic");
    start_motion(app.world_mut(), false);
    let expected = app.world().get::<MoveQueue>(entity).unwrap().snapshot();
    let source = app
        .world()
        .resource::<AssetServer>()
        .load::<Image>(resolve_png("CharSet", "Poses2"));
    app.world_mut()
        .get_mut::<Player>(entity)
        .unwrap()
        .set_graphic("Poses2".into(), 4);
    app.update();
    assert_eq!(
        app.world().get::<MoveQueue>(entity).unwrap().snapshot(),
        expected
    );
    assert_eq!(app.world().get::<Sprite>(entity).unwrap().image, source);
}
