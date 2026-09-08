use super::*;
use amnezia_data::Event;

#[test]
fn chasing_enemy_starts_its_touch_event_when_it_reaches_the_hero() {
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
    ));
    app.update();
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(1));
}

/// A passability closure that blocks the listed directions and allows the rest.
fn block(blocked: &'static [u32]) -> impl Fn(u32) -> bool {
    move |dir| !blocked.contains(&dir)
}

#[test]
fn frequency_gates_the_step_cadence() {
    // Higher frequency ⇒ shorter delay ⇒ more frequent steps.
    assert!(stop_frames(1) > stop_frames(3));
    assert!(stop_frames(3) > stop_frames(6));
    assert_eq!(stop_frames(3), 64); // 1 << (9 - 3)
    assert_eq!(stop_frames(8), 0); // fastest: no wait
    // Out-of-range frequencies clamp into 1..=8 rather than overflow-shifting.
    assert_eq!(stop_frames(0), stop_frames(1));
    assert_eq!(stop_frames(99), stop_frames(8));
}

#[test]
fn speed_scales_the_tween_by_powers_of_two() {
    // Each slower speed doubles the per-tile time; speed 4 is the hero anchor.
    assert!(step_secs_for_speed(3) > step_secs_for_speed(4));
    assert!((step_secs_for_speed(4) / step_secs_for_speed(5) - 2.0).abs() < 1e-6);
    assert_eq!(step_secs_for_speed(0), step_secs_for_speed(1)); // clamps low
    assert_eq!(step_secs_for_speed(9), step_secs_for_speed(6)); // clamps high
}

#[test]
fn random_mover_steps_when_open_and_turns_when_blocked() {
    // move_type 1 takes the pre-drawn direction: steps it when passable,
    // otherwise just turns to face it (the RM2000 turn/idle fallback).
    assert_eq!(
        decide(1, DIR_DOWN, 2, 2, 9, 9, DIR_RIGHT, block(&[])),
        Decision::Step(DIR_RIGHT)
    );
    assert_eq!(
        decide(1, DIR_DOWN, 2, 2, 9, 9, DIR_UP, block(&[DIR_UP])),
        Decision::Face(DIR_UP)
    );
}

#[test]
fn pace_mover_reverses_at_a_block() {
    // Vertical pacer facing Down with Down blocked reverses and steps Up.
    assert_eq!(
        decide(2, DIR_DOWN, 2, 2, 2, 2, 0, block(&[DIR_DOWN])),
        Decision::Step(DIR_UP)
    );
    // Boxed in on both ends: it still reverses its facing (to Up), no step.
    assert_eq!(
        decide(2, DIR_DOWN, 2, 2, 2, 2, 0, block(&[DIR_DOWN, DIR_UP])),
        Decision::Face(DIR_UP)
    );
    // Horizontal pacer keeps its reverse heading when already facing Left.
    assert_eq!(
        decide(3, DIR_LEFT, 2, 2, 2, 2, 0, block(&[])),
        Decision::Step(DIR_LEFT)
    );
}

#[test]
fn toward_mover_steps_closer_and_away_mover_steps_off() {
    // Player three tiles to the right: toward steps Right (closer), away Left.
    assert_eq!(
        decide(4, DIR_DOWN, 2, 2, 5, 2, 0, block(&[])),
        Decision::Step(DIR_RIGHT)
    );
    assert_eq!(
        decide(5, DIR_DOWN, 2, 2, 5, 2, 0, block(&[])),
        Decision::Step(DIR_LEFT)
    );
    // Diagonal: the dominant axis (vertical here, since |dy| >= |dx|) wins.
    assert_eq!(toward_candidates(1, 3), vec![DIR_DOWN, DIR_RIGHT]);
    assert_eq!(away_candidates(1, 3), vec![DIR_UP, DIR_LEFT]);
}

#[test]
fn passability_blocks_a_step_no_wall_walking() {
    // Toward the player but every neighbour blocked: it faces, never steps.
    let d = decide(
        4,
        DIR_DOWN,
        2,
        2,
        5,
        2,
        0,
        block(&[DIR_UP, DIR_DOWN, DIR_LEFT, DIR_RIGHT]),
    );
    assert!(matches!(d, Decision::Face(_)));
    assert!(!matches!(d, Decision::Step(_)));
}

fn app_with_mover(move_type: u32, timer: f32) -> App {
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
            timer,
            rng: 1,
        },
    ));
    app
}

fn event_tile(app: &App) -> (u32, u32) {
    let ev = &app.world().resource::<MapEvents>().events[0];
    (ev.x, ev.y)
}

#[test]
fn ready_toward_mover_updates_logical_tile_and_queues_the_step() {
    // A toward-hero mover whose timer is already up steps Right toward the
    // player at (5,2): the logical MapEvents tile advances to (3,2), the
    // sprite faces Right, and its queue holds the tween.
    let mut app = app_with_mover(4, 0.0);
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
    // The same mover, but a long countdown gates it: no step this frame.
    let mut app = app_with_mover(4, 100.0);
    app.update();
    assert_eq!(event_tile(&app), (2, 2));
    let world = app.world_mut();
    let mut q = world.query::<&MoveQueue>();
    assert!(!q.single(world).unwrap().busy());
}
