use super::*;
use crate::world::stop_clock::step as stop_frames;
use amnezia_data::Event;

mod contacts;
mod decisions;

fn chasing_app() -> App {
    let mut page = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0001.ron",
        crate::assets::asset_root()
    ))
    .events[0]
        .pages[0]
        .clone();
    page.condition = default();
    page.trigger = 2;
    page.layer = 1;
    page.commands = vec![amnezia_data::EventCommand {
        code: 11410,
        indent: 0,
        string: String::new(),
        params: vec![100],
    }];
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(MapData::for_test(10, 10))
        .insert_resource(MapEvents {
            events: vec![Event {
                id: 1,
                x: 5,
                y: 6,
                name: String::new(),
                pages: vec![page],
            }],
        })
        .init_resource::<Switches>()
        .init_resource::<Variables>()
        .init_resource::<Party>()
        .init_resource::<Inventory>()
        .init_resource::<Dialogue>()
        .init_resource::<Fade>()
        .init_resource::<RunningEvent>()
        .init_resource::<MenuOpen>()
        .init_resource::<ShopOpen>()
        .init_resource::<BattleActive>()
        .init_resource::<GameOverActive>()
        .init_resource::<super::super::TouchEvents>()
        .insert_resource(TitleActive(false))
        .add_systems(
            Update,
            (autonomous_movement, super::super::touch::trigger).chain(),
        );
    app.world_mut().spawn((
        Player {
            tile_x: 5,
            tile_y: 5,
            dir: DIR_DOWN,
            frame: 1,
            charset: String::new(),
            index: 0,
        },
        MoveQueue::default(),
        RouteStepper::default(),
    ));
    app.world_mut().spawn((
        EventSprite {
            id: 1,
            tile_x: 5,
            tile_y: 6,
            dir: DIR_UP,
            frame: 1,
            charset: String::new(),
            index: 0,
            layer: 1,
        },
        MoveQueue::default(),
        AutoMove::new(4, 8, 4, 1),
        RouteStepper::from_page(&amnezia_data::MoveRouteDef::default(), 4, 8),
    ));
    app
}

#[test]
fn chasing_enemy_starts_its_touch_event_when_it_reaches_the_hero() {
    let mut app = chasing_app();
    app.update();
    assert_eq!(app.world().resource::<RunningEvent>().queued_ids(), vec![1]);
}

#[test]
fn a_parallel_message_without_a_foreground_event_does_not_freeze_npcs() {
    let mut app = chasing_app();
    let world = app.world_mut();
    world
        .query::<&mut Player>()
        .single_mut(world)
        .unwrap()
        .tile_y = 4;
    world.resource_mut::<Dialogue>().active = true;
    app.update();
    assert_eq!(app.world().resource::<MapEvents>().events[0].y, 5);
}

#[test]
fn npc_collision_can_queue_behind_a_parallel_choice() {
    let mut app = chasing_app();
    let mut choice = crate::choice::Choice::default();
    choice.open(vec!["Igen".into()], 0, 0);
    app.insert_resource(choice);
    app.update();
    assert!(!app.world().resource::<RunningEvent>().active());
    assert_eq!(app.world().resource::<RunningEvent>().queued_ids(), vec![1]);
    assert!(app.world().resource::<crate::choice::Choice>().active());
}

#[test]
fn continue_events_allows_other_npcs_to_walk_but_keeps_the_speaking_npc_paused() {
    for (continues, event_id, menu, moves) in [
        (false, 2, false, false),
        (true, 2, false, true),
        (true, 1, false, false),
        (true, 2, true, false),
    ] {
        let mut app = chasing_app();
        app.insert_resource(crate::dialogue::MessageOptions {
            fixed: false,
            continue_events: continues,
        });
        let world = app.world_mut();
        world
            .query::<&mut Player>()
            .single_mut(world)
            .unwrap()
            .tile_y = 4;
        world.resource_mut::<RunningEvent>().start(
            event_id,
            vec![amnezia_data::EventCommand {
                code: 11410,
                indent: 0,
                string: String::new(),
                params: vec![100],
            }],
        );
        world.resource_mut::<Dialogue>().active = true;
        world.resource_mut::<MenuOpen>().0 = menu;
        app.update();
        assert_eq!(
            app.world().resource::<MapEvents>().events[0].y,
            if moves { 5 } else { 6 }
        );
    }
}

#[test]
fn continue_events_resumes_an_autonomous_custom_route_during_another_events_message() {
    let mut app = chasing_app();
    app.insert_resource(crate::dialogue::MessageOptions {
        fixed: false,
        continue_events: true,
    });
    app.add_message::<crate::audio::AudioRequest>();
    app.add_systems(
        Update,
        super::super::route::route_events.before(autonomous_movement),
    );
    let world = app.world_mut();
    let npc = world
        .query_filtered::<Entity, With<EventSprite>>()
        .single(world)
        .unwrap();
    world.entity_mut(npc).insert((
        Sprite::default(),
        RouteStepper::from_page(
            &amnezia_data::MoveRouteDef {
                commands: vec![amnezia_data::MoveCommandDef {
                    code: 2,
                    string: String::new(),
                    params: vec![],
                }],
                repeat: false,
                skippable: false,
            },
            4,
            8,
        ),
    ));
    world.resource_mut::<RunningEvent>().start(
        2,
        vec![amnezia_data::EventCommand {
            code: 11410,
            indent: 0,
            string: String::new(),
            params: vec![100],
        }],
    );
    world.resource_mut::<Dialogue>().active = true;
    app.update();
    assert_eq!(app.world().resource::<MapEvents>().events[0].y, 7);
}

#[test]
fn chasing_npc_uses_the_short_path_and_touch_collision_across_a_seam() {
    for hero_y in [8, 9] {
        let mut app = chasing_app();
        let world = app.world_mut();
        world.resource_mut::<MapData>().scroll_type = 1;
        world
            .query::<&mut Player>()
            .single_mut(world)
            .unwrap()
            .tile_y = hero_y;
        world
            .query::<&mut EventSprite>()
            .single_mut(world)
            .unwrap()
            .tile_y = 0;
        world.resource_mut::<MapEvents>().events[0].y = 0;
        app.update();
        let world = app.world();
        assert_eq!(
            world.resource::<RunningEvent>().queued_ids(),
            if hero_y == 9 { vec![1] } else { vec![] }
        );
        assert_eq!(
            world.resource::<MapEvents>().events[0].y,
            if hero_y == 9 { 0 } else { 9 }
        );
    }
}

#[test]
fn autonomous_movement_keeps_scripted_through_facing_and_speed() {
    let mut app = chasing_app();
    let world = app.world_mut();
    let npc = world
        .query_filtered::<Entity, With<EventSprite>>()
        .single(world)
        .unwrap();
    world.entity_mut(npc).insert(RouteStepper::from_page(
        &amnezia_data::MoveRouteDef::default(),
        4,
        8,
    ));
    let (mut character, mut queue, mut route) = world
        .query::<(&mut EventSprite, &mut MoveQueue, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    character.dir = DIR_DOWN;
    route.force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 36, 26, 29]));
    crate::world::drive_route(
        &mut *character,
        &mut queue,
        &mut route,
        (5, 5),
        |_, _, _, _, through| through,
    );
    assert!(!route.forced());
    app.update();
    assert!(!app.world().resource::<RunningEvent>().active());
    let world = app.world_mut();
    let (mut character, mut queue) = world
        .query::<(&mut EventSprite, &mut MoveQueue)>()
        .single_mut(world)
        .unwrap();
    assert!(queue.busy());
    let data = MapData::for_test(10, 10);
    for _ in 0..8 {
        queue.advance(&mut *character, &data, 1.0 / 60.0);
    }
    assert!(queue.busy());
    for _ in 0..10 {
        queue.advance(&mut *character, &data, 1.0 / 60.0);
    }
    assert!(!queue.busy());
    assert_eq!(
        (character.tile_x, character.tile_y, character.dir),
        (5, 5, DIR_DOWN)
    );
}

#[test]
fn frequency_gates_the_step_cadence() {
    assert!(stop_frames(1) > stop_frames(3));
    assert!(stop_frames(3) > stop_frames(6));
    assert_eq!(stop_frames(3), 64);
    assert_eq!(stop_frames(8), 0);
    assert_eq!(stop_frames(0), stop_frames(1));
    assert_eq!(stop_frames(99), stop_frames(8));
}

#[test]
fn speed_scales_the_tween_by_powers_of_two() {
    assert!(step_secs_for_speed(3) > step_secs_for_speed(4));
    assert!((step_secs_for_speed(4) / step_secs_for_speed(5) - 2.0).abs() < 1e-6);
    assert_eq!(step_secs_for_speed(0), step_secs_for_speed(1));
    assert_eq!(step_secs_for_speed(9), step_secs_for_speed(6));
}

fn app_with_mover(move_type: u32, stops_left: u32) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(MapData::for_test(10, 10));
    app.insert_resource(MapEvents {
        events: vec![Event {
            id: 1,
            x: 2,
            y: 2,
            name: String::new(),
            pages: Vec::new(),
        }],
    });
    app.init_resource::<Switches>();
    app.init_resource::<Variables>();
    app.init_resource::<Party>();
    app.init_resource::<Inventory>();
    app.init_resource::<Dialogue>();
    app.init_resource::<Fade>();
    app.init_resource::<MenuOpen>();
    app.init_resource::<ShopOpen>();
    app.init_resource::<BattleActive>();
    app.init_resource::<GameOverActive>();
    app.init_resource::<RunningEvent>();
    app.insert_resource(TitleActive(false));
    app.add_systems(Update, autonomous_movement);
    app.world_mut().spawn(Player {
        tile_x: 5,
        tile_y: 2,
        dir: DIR_DOWN,
        frame: 1,
        charset: "C".into(),
        index: 0,
    });
    let mut route = RouteStepper::from_page(&amnezia_data::MoveRouteDef::default(), 3, 3);
    route.set_stop_maximum(stops_left);
    app.world_mut().spawn((
        EventSprite {
            id: 1,
            tile_x: 2,
            tile_y: 2,
            dir: DIR_DOWN,
            frame: 1,
            charset: "C".into(),
            index: 0,
            layer: 1,
        },
        MoveQueue::default(),
        AutoMove {
            move_type,
            frequency: 3,
            speed: 3,
            legacy_timer: 0.0,
            rng: 1,
        },
        route,
    ));
    app
}

fn event_tile(app: &App) -> (u32, u32) {
    let ev = &app.world().resource::<MapEvents>().events[0];
    (ev.x, ev.y)
}

#[test]
fn ready_toward_mover_updates_logical_tile_and_queues_the_step() {
    let mut app = app_with_mover(4, 0);
    app.update();
    assert_eq!(event_tile(&app), (3, 2));
    let world = app.world_mut();
    let mut q = world.query::<(&EventSprite, &MoveQueue)>();
    let (sprite, queue) = q.single(world).unwrap();
    assert_eq!(sprite.dir, DIR_RIGHT);
    assert!(queue.busy());
}

#[test]
fn mover_with_time_remaining_stays_put() {
    let mut app = app_with_mover(4, 100);
    app.update();
    assert_eq!(event_tile(&app), (2, 2));
    let world = app.world_mut();
    let mut q = world.query::<&MoveQueue>();
    assert!(!q.single(world).unwrap().busy());
}
