use super::npcs::npc;
use super::*;
use crate::world::{AutoMove, EventSprite, RouteAction};

fn append(digit: i32) -> Vec<EventCommand> {
    vec![
        cmd(10220, 0, vec![0, 1, 1, 3, 0, 10]),
        cmd(10220, 0, vec![0, 1, 1, 1, 0, digit]),
    ]
}

fn event(id: u32, trigger: u32, y: u32, layer: u32) -> Event {
    let mut event = map_event(id, trigger, append(id as i32));
    event.x = 5;
    event.y = y;
    event.pages[0].layer = layer;
    event
}

fn press(app: &mut App, keys: &[KeyCode]) {
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    *input = default();
    for &key in keys {
        input.press(key);
    }
    app.update();
}

fn value(app: &App) -> i32 {
    app.world().resource::<Variables>().get(1)
}

#[test]
fn decision_queues_all_front_touches_floor_actions_and_front_actions_in_id_order() {
    let mut app = app();
    crate::dialogue::testing::register_actions(&mut app);
    app.insert_resource(MapEvents {
        events: vec![
            event(4, 1, 6, 1),
            event(3, 0, 5, 0),
            event(2, 0, 6, 1),
            event(1, 2, 6, 1),
            event(5, 1, 5, 0),
        ],
    });
    press(&mut app, &[KeyCode::Enter]);
    assert_eq!(value(&app), 1234);
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn an_empty_action_page_does_not_stop_the_counter_search() {
    let mut app = app();
    crate::dialogue::testing::register_actions(&mut app);
    let mut empty = event(1, 0, 6, 1);
    empty.pages[0].commands.clear();
    app.insert_resource(MapEvents {
        events: vec![empty, event(2, 0, 7, 1)],
    });
    app.world_mut()
        .resource_mut::<MapData>()
        .set_counter_for_test(5, 6);
    press(&mut app, &[KeyCode::Enter]);
    assert_eq!(value(&app), 2);
}

#[test]
fn a_blocked_step_queues_every_touch_without_suppressing_the_same_decision() {
    for decision in [false, true] {
        let mut app = app();
        crate::dialogue::testing::register_actions(&mut app);
        app.insert_resource(MapEvents {
            events: vec![event(3, 0, 6, 1), event(2, 1, 6, 1), event(1, 2, 6, 1)],
        });
        press(
            &mut app,
            if decision {
                &[KeyCode::ArrowDown, KeyCode::Enter]
            } else {
                &[KeyCode::ArrowDown]
            },
        );
        assert_eq!(value(&app), if decision { 123 } else { 12 });
    }
}

#[test]
fn an_idle_floor_collision_is_checked_before_directional_input() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![event(1, 2, 5, 0)],
    });
    press(&mut app, &[KeyCode::ArrowRight]);
    assert_eq!(hero_x(&mut app), 5);
    assert_eq!(value(&app), 1);
}

#[test]
fn arriving_during_a_foreground_wait_retains_all_floor_touches_for_later() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![event(2, 1, 6, 0), event(1, 1, 6, 0)],
    });
    let world = app.world_mut();
    world
        .resource_mut::<RunningEvent>()
        .start(0, vec![cmd(11410, 0, vec![100])]);
    let mut queue = world.query::<&mut MoveQueue>().single_mut(world).unwrap();
    queue.set_step_secs(2.0 / 60.0);
    queue.push_step(RouteAction::Step {
        dx: 0,
        dy: 1,
        face: crate::tiles::DIR_DOWN,
    });
    app.update();
    app.update();
    assert_eq!(value(&app), 0);
    assert!(app.world().resource::<RunningEvent>().waiting());
    app.world_mut().resource_mut::<RunningEvent>().frame.wait = 0.0;
    app.update();
    assert_eq!(value(&app), 12);
}

fn floor_npc(app: &mut App, id: u32) -> Entity {
    let entity = npc(app, id, 5, append(id as i32), &[]);
    {
        let mut events = app.world_mut().resource_mut::<MapEvents>();
        let event = events
            .events
            .iter_mut()
            .find(|event| event.id == id)
            .unwrap();
        event.y = 5;
        event.pages[0].trigger = 2;
        event.pages[0].layer = 0;
    }
    let mut sprite = app.world_mut().get_mut::<EventSprite>(entity).unwrap();
    sprite.tile_y = 5;
    sprite.layer = 0;
    entity
}

#[test]
fn an_npc_floor_collision_pauses_itself_before_autonomy_even_during_a_parallel_prompt() {
    let mut app = app();
    let first = floor_npc(&mut app, 1);
    app.world_mut()
        .entity_mut(first)
        .insert(AutoMove::new(3, 8, 4, 1));
    app.world_mut()
        .resource_mut::<Choice>()
        .open(vec!["Yes".into()], 0, 0);
    app.update();
    assert_eq!(app.world().get::<EventSprite>(first).unwrap().tile_x, 5);
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(1));
    assert_eq!(value(&app), 0);
    app.world_mut().resource_mut::<Choice>().active = false;
    app.update();
    assert_eq!(value(&app), 1);
}

#[test]
fn queued_npc_collisions_do_not_freeze_later_characters_before_foreground_execution() {
    let mut app = app();
    floor_npc(&mut app, 1);
    let second = npc(&mut app, 2, 6, vec![], &[]);
    app.world_mut()
        .entity_mut(second)
        .insert(AutoMove::new(3, 8, 4, 2));
    app.update();
    assert_eq!(app.world().get::<EventSprite>(second).unwrap().tile_x, 7);
    assert_eq!(value(&app), 1);
}

#[test]
fn boats_and_ships_can_talk_when_disembarking_fails_but_airships_cannot() {
    for vehicle in 0..3 {
        let mut app = app();
        crate::dialogue::testing::register_actions(&mut app);
        app.add_plugins(crate::vehicles::VehiclePlugin)
            .init_resource::<crate::audio::CurrentBgm>();
        app.insert_resource(MapEvents {
            events: vec![event(1, 0, 6, 1)],
        });
        {
            let mut vehicles = app.world_mut().resource_mut::<crate::vehicles::Vehicles>();
            vehicles.set_location(vehicle, 0, 5, 5);
            vehicles.save.vehicles[vehicle].dir = crate::tiles::DIR_DOWN;
            vehicles.save.riding = Some(vehicle);
        }
        press(&mut app, &[KeyCode::Enter]);
        assert_eq!(value(&app), if vehicle == 2 { 0 } else { 1 });
    }
}

#[test]
fn an_npc_floor_collision_does_not_wait_for_the_heros_tween_to_finish() {
    let mut app = app();
    let world = app.world_mut();
    world
        .query::<&mut Player>()
        .single_mut(world)
        .unwrap()
        .tile_x = 4;
    let mut queue = world.query::<&mut MoveQueue>().single_mut(world).unwrap();
    queue.set_step_secs(1.0);
    queue.push_step(RouteAction::Step {
        dx: 1,
        dy: 0,
        face: crate::tiles::DIR_RIGHT,
    });
    app.update();
    floor_npc(&mut app, 1);
    app.update();
    assert_eq!(value(&app), 1);
    let world = app.world_mut();
    assert!(
        world
            .query_filtered::<&MoveQueue, With<Player>>()
            .single(world)
            .unwrap()
            .busy()
    );
}

#[test]
fn touches_queued_by_the_current_move_attempt_do_not_prevent_boarding() {
    let mut app = app();
    app.add_plugins(crate::vehicles::VehiclePlugin)
        .init_resource::<crate::audio::CurrentBgm>();
    app.insert_resource(MapEvents {
        events: vec![event(1, 1, 6, 1)],
    });
    app.world_mut()
        .resource_mut::<crate::vehicles::Vehicles>()
        .set_location(0, 0, 5, 6);
    press(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]);
    assert_eq!(
        app.world()
            .resource::<crate::vehicles::Vehicles>()
            .save
            .riding,
        Some(0)
    );
    assert_eq!(value(&app), 1);
}

#[test]
fn a_moving_boat_cannot_accept_actions_or_idle_floor_collisions() {
    let mut app = app();
    crate::dialogue::testing::register_actions(&mut app);
    app.add_plugins(crate::vehicles::VehiclePlugin)
        .init_resource::<crate::audio::CurrentBgm>();
    {
        let mut vehicles = app.world_mut().resource_mut::<crate::vehicles::Vehicles>();
        vehicles.set_location(0, 0, 5, 5);
        vehicles.save.riding = Some(0);
    }
    press(&mut app, &[KeyCode::ArrowDown]);
    app.insert_resource(MapEvents {
        events: vec![event(1, 0, 6, 0), event(2, 2, 6, 0)],
    });
    press(&mut app, &[KeyCode::Enter]);
    assert_eq!(value(&app), 0);
    assert!(
        app.world()
            .resource::<crate::vehicles::Vehicles>()
            .rider_moving()
    );
    assert!(!app.world().resource::<RunningEvent>().active());
}
