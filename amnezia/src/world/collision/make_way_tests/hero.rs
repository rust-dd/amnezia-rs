use super::*;
use crate::player::{CameraPan, Player};

mod callbacks;
mod logical;

fn at(id: u32, x: u32, y: u32, page: amnezia_data::EventPage) -> amnezia_data::Event {
    let mut event = event(id, x, vec![page]);
    event.y = y;
    event
}

fn hero_entity(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap()
}

fn move_hero(app: &mut App, forced: bool, direction: u32) {
    let hero = hero_entity(app);
    if forced {
        app.world_mut()
            .get_mut::<RouteStepper>(hero)
            .unwrap()
            .force_route(RouteStepper::from_move_event(&[
                10001,
                8,
                0,
                0,
                direction as i32,
            ]));
    } else {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(match direction {
                0 => KeyCode::ArrowUp,
                1 => KeyCode::ArrowRight,
                2 => KeyCode::ArrowDown,
                _ => KeyCode::ArrowLeft,
            });
    }
}

#[test]
fn npc_collisions_update_manual_and_forced_heroes_before_later_parallel_queries() {
    for forced in [false, true] {
        for layer in 0..=2 {
            let mut mover = page(vec![command(1, 0)]);
            mover.layer = layer;
            let mut observer = page(vec![]);
            observer.trigger = 4;
            observer.commands = vec![EventCommand {
                params: vec![0, 1, 1, 0, 6, 10001, 1],
                ..counter()
            }];
            let mut app = app(vec![at(1, 4, 5, mover), at(2, 9, 1, observer)], false);
            let hero = hero_entity(&mut app);
            move_hero(&mut app, forced, 1);
            app.update();
            assert_eq!(app.world().resource::<crate::state::Variables>().get(1), 6);
            let npc = entity(&mut app, 1);
            assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile(), (5, 5));
            let character = app.world().get::<Player>(hero).unwrap();
            assert_eq!(character.tile(), (6, 5));
            let data = app.world().resource::<MapData>();
            let queue = app.world().get::<MoveQueue>(hero).unwrap();
            assert_eq!(
                queue.ground_position(character, data),
                Vec2::from(data.tile_center(5, 5)) + Vec2::X * 2.0
            );
        }
    }
}

#[test]
fn repeated_hero_visits_advance_pan_each_time_but_character_clocks_only_once() {
    let mut app = app(
        vec![
            at(1, 4, 5, page(vec![command(1, 0)])),
            at(2, 5, 4, page(vec![command(2, 0)])),
        ],
        false,
    );
    app.insert_resource(MapData::for_test(40, 30));
    app.world_mut().spawn((
        crate::world::MainCamera,
        Transform::default(),
        Projection::Orthographic(OrthographicProjection {
            area: Rect::new(-160.0, -120.0, 160.0, 120.0),
            ..OrthographicProjection::default_2d()
        }),
    ));
    let mut pan = app.world_mut().resource_mut::<CameraPan>();
    pan.position = Some(Vec2::ZERO);
    pan.locked = true;
    pan.target = Vec2::X * 16.0;
    pan.speed = 60.0;
    let hero = hero_entity(&mut app);
    let stopped = app.world().get::<RouteStepper>(hero).unwrap().stop_count();
    app.update();
    assert_eq!(app.world().resource::<CameraPan>().offset.x, 3.0);
    assert_eq!(
        app.world().get::<RouteStepper>(hero).unwrap().stop_count(),
        stopped + 1
    );
}

#[test]
fn an_early_hero_update_checks_decision_actions_before_the_parallel_phase_ends() {
    #[derive(Resource, Default)]
    struct Observed(Vec<u32>);
    let mut action = page(vec![]);
    action.commands = vec![counter()];
    let mut app = app(
        vec![at(1, 4, 5, page(vec![command(1, 0)])), at(2, 5, 6, action)],
        false,
    );
    crate::dialogue::testing::register_actions(&mut app);
    app.init_resource::<Observed>().add_systems(
        Update,
        (|running: Res<crate::interpreter::RunningEvent>, mut seen: ResMut<Observed>| {
            seen.0 = running.queued_ids();
        })
        .after(crate::interpreter::ParallelStep)
        .before(crate::player::PlayerInput),
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(app.world().resource::<Observed>().0, [2]);
    assert_eq!(app.world().resource::<crate::state::Variables>().get(1), 1);
}

#[test]
fn the_hero_can_extend_an_early_movement_chain_through_a_later_npc() {
    for forced in [false, true] {
        let mut other = page(vec![command(1, 0)]);
        other.trigger = 4;
        other.commands = vec![counter()];
        let mut app = app(
            vec![at(1, 4, 5, page(vec![command(1, 0)])), at(2, 6, 5, other)],
            false,
        );
        move_hero(&mut app, forced, 1);
        app.update();
        assert_eq!(app.world().resource::<crate::state::Variables>().get(1), 2);
        let hero = hero_entity(&mut app);
        assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (6, 5));
        for (id, from) in [(1, 4), (2, 6)] {
            let npc = entity(&mut app, id);
            let character = app.world().get::<EventSprite>(npc).unwrap();
            assert_eq!(character.tile(), (from + 1, 5));
            let queue = app.world().get::<MoveQueue>(npc).unwrap();
            let data = app.world().resource::<MapData>();
            assert_eq!(
                queue.ground_position(character, data),
                Vec2::from(data.tile_center(from, 5)) + Vec2::X * 2.0
            );
        }
    }
}

#[test]
fn a_hero_npc_collision_cycle_revisits_scripts_without_reentering_character_updates() {
    for forced in [false, true] {
        let mut mover = page(vec![command(1, 0)]);
        mover.trigger = 4;
        mover.commands = vec![counter()];
        let mut app = app(vec![at(1, 4, 5, mover)], false);
        move_hero(&mut app, forced, 3);
        let hero = hero_entity(&mut app);
        let stopped = app.world().get::<RouteStepper>(hero).unwrap().stop_count();
        app.update();
        assert_eq!(app.world().resource::<crate::state::Variables>().get(1), 2);
        assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (5, 5));
        assert_eq!(
            app.world().get::<RouteStepper>(hero).unwrap().stop_count(),
            stopped + 1
        );
        let npc = entity(&mut app, 1);
        assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile(), (4, 5));
        assert_eq!(
            app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
            1
        );
    }
}

#[test]
fn normal_hero_movement_revisits_a_processed_targets_parallel_script() {
    for forced in [false, true] {
        let mut other = page(vec![]);
        other.trigger = 4;
        other.commands = vec![counter()];
        let mut app = app(vec![at(1, 6, 5, other)], false);
        move_hero(&mut app, forced, 1);
        app.update();
        assert_eq!(app.world().resource::<crate::state::Variables>().get(1), 2);
        let hero = hero_entity(&mut app);
        assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (5, 5));
        let npc = entity(&mut app, 1);
        assert_eq!(
            app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
            1
        );
    }
}

#[test]
fn forced_hero_and_npc_updates_can_make_way_during_a_foreground_cutscene() {
    let mut app = app(vec![at(1, 4, 5, page(vec![]))], false);
    let npc = entity(&mut app, 1);
    app.world_mut()
        .get_mut::<RouteStepper>(npc)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 1]));
    move_hero(&mut app, true, 1);
    app.world_mut()
        .resource_mut::<crate::interpreter::RunningEvent>()
        .start(
            0,
            vec![EventCommand {
                code: 11410,
                params: vec![100],
                ..counter()
            }],
        );
    app.update();
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile(), (5, 5));
    let hero = hero_entity(&mut app);
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (6, 5));
}

#[test]
fn a_through_npc_does_not_pull_the_heroes_update_ahead_of_parallel_queries() {
    let mut observer = page(vec![]);
    observer.trigger = 4;
    observer.commands = vec![EventCommand {
        params: vec![0, 1, 1, 0, 6, 10001, 1],
        ..counter()
    }];
    let mut app = app(
        vec![
            at(1, 4, 5, page(vec![command(36, 0), command(1, 0)])),
            at(2, 9, 1, observer),
        ],
        false,
    );
    move_hero(&mut app, false, 1);
    app.update();
    assert_eq!(app.world().resource::<crate::state::Variables>().get(1), 5);
    let hero = hero_entity(&mut app);
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (6, 5));
}
