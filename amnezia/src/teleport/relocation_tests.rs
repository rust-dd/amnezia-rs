use super::*;
use crate::world::{Character, RouteAction};

fn reposition(
    data: Res<MapData>,
    mut players: Query<(
        &mut Player,
        &mut Transform,
        &mut MoveQueue,
        &mut RouteStepper,
    )>,
) {
    reposition_hero(&mut players, &data, 8, 9);
}

fn relocated(jumping: bool) -> (App, Entity, RouteStepper) {
    let data = MapData::for_test(40, 30);
    let mut player = Player {
        tile_x: 2,
        tile_y: 3,
        dir: crate::tiles::DIR_RIGHT,
        frame: 3,
        charset: "Chara1".into(),
        index: 0,
    };
    let mut queue = MoveQueue::default();
    queue.use_character_motion(2, player.dir);
    queue.begin_from(
        &mut player,
        &data,
        (2, 3),
        if jumping {
            RouteAction::Jump {
                dx: 2,
                dy: 0,
                face: crate::tiles::DIR_RIGHT,
            }
        } else {
            RouteAction::Step {
                dx: 1,
                dy: 0,
                face: crate::tiles::DIR_RIGHT,
            }
        },
    );
    let route = RouteStepper::from_move_event(&[10001, 4, 1, 0, 1, 32, 9]).with_speed(2);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).insert_resource(data);
    let hero = app
        .world_mut()
        .spawn((player, queue, route.clone(), Transform::default()))
        .id();
    app.world_mut().run_system_cached(reposition).unwrap();
    (app, hero, route)
}

#[test]
fn hero_relocation_keeps_its_live_route_speed_and_animation() {
    let (app, hero, route) = relocated(false);
    assert_eq!(app.world().get::<RouteStepper>(hero).unwrap(), &route);
    assert_eq!(app.world().get::<Player>(hero).unwrap().frame, 3);
    assert!(!app.world().get::<MoveQueue>(hero).unwrap().busy());
}

#[test]
fn hero_relocation_keeps_a_zero_remaining_jump_at_the_new_position() {
    let (app, hero, _) = relocated(true);
    let player = app.world().get::<Player>(hero).unwrap();
    let queue = app.world().get::<MoveQueue>(hero).unwrap();
    assert!(queue.jumping());
    assert_eq!(player.tile(), (8, 9));
    let data = app.world().resource::<MapData>();
    assert_eq!(
        queue.render_position(player, data),
        Vec2::from(data.tile_center(8, 9))
    );
}

#[test]
fn hero_relocation_resumes_the_live_route_without_replaying_its_modifiers() {
    let mut app = crate::world::test_support::app(vec![], false);
    let hero = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<RouteStepper>(hero)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[
            10001, 8, 0, 0, 36, 40, 29, 1, 32, 8,
        ]));
    app.update();
    assert!(app.world().get::<MoveQueue>(hero).unwrap().busy());
    let route = app.world().get::<RouteStepper>(hero).unwrap().clone();
    assert_eq!(route.speed(), 3);
    assert!(route.through());
    assert!(!app.world().resource::<Switches>().get(8));
    app.world_mut().run_system_cached(reposition).unwrap();
    assert_eq!(app.world().get::<RouteStepper>(hero).unwrap(), &route);
    app.update();
    assert!(app.world().resource::<Switches>().get(8));
    let resumed = app.world().get::<RouteStepper>(hero).unwrap();
    assert_eq!(resumed.speed(), 3);
    assert_eq!(resumed.alpha(), route.alpha());
    assert!(resumed.through());
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (8, 9));
}
