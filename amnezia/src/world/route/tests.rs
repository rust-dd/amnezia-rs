use super::*;
use crate::battle::BattleActive;
use crate::dialogue::Dialogue;
use crate::gameover::GameOverActive;
use crate::interpreter::RunningEvent;
use crate::menu::MenuOpen;
use crate::shop::ShopOpen;
use crate::teleport::Fade;
use crate::tiles::DIR_DOWN;
use crate::title::TitleActive;
use amnezia_data::{Event, EventCommand, MoveCommandDef, MoveRouteDef};

#[test]
fn forced_routes_check_the_wrapped_destination_for_hero_collision() {
    let mut data = MapData::for_test(140, 140);
    data.scroll_type = 3;
    let state = (
        &Switches::default(),
        &Variables::default(),
        &Party::default(),
        &Inventory::default(),
    );
    for jumping in [false, true] {
        assert!(!tile_open(
            0,
            2,
            -1,
            0,
            jumping,
            (1, 1),
            (139, 2),
            &data,
            &MapEvents::default(),
            state
        ));
        assert!(tile_open(
            0,
            2,
            -1,
            0,
            jumping,
            (1, 0),
            (139, 2),
            &data,
            &MapEvents::default(),
            state
        ));
        assert!(tile_open(
            0,
            2,
            -1,
            0,
            jumping,
            (1, 1),
            (138, 2),
            &data,
            &MapEvents::default(),
            state
        ));
    }
}

/// A `RunningEvent` mid-execution — a one-command frame is enough to make
/// `active()` hold, standing in for a cutscene that is still running.
fn running_event() -> RunningEvent {
    let mut running = RunningEvent::default();
    running.start(
        1,
        vec![EventCommand {
            code: 10110,
            indent: 0,
            string: String::new(),
            params: Vec::new(),
        }],
    );
    running
}

#[test]
fn move_type_six_npc_follows_its_route() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(MapData::for_test(10, 10));
    app.insert_resource(MapEvents {
        events: vec![Event {
            id: 1,
            x: 5,
            y: 5,
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
    app.add_message::<AudioRequest>();
    app.add_systems(Update, route_events);
    app.world_mut().spawn(Player {
        tile_x: 0,
        tile_y: 0,
        dir: DIR_DOWN,
        frame: 1,
        charset: "C".into(),
        index: 0,
    });
    let route = MoveRouteDef {
        commands: vec![MoveCommandDef {
            code: 2,
            params: Vec::new(),
            string: String::new(),
        }],
        repeat: true,
        skippable: false,
    };
    app.world_mut().spawn((
        EventSprite {
            id: 1,
            tile_x: 5,
            tile_y: 5,
            dir: DIR_DOWN,
            frame: 1,
            charset: "C".into(),
            index: 0,
            layer: 1,
        },
        MoveQueue::default(),
        RouteStepper::from_page(&route, 6, 8),
        Sprite::default(),
    ));
    app.update();
    let world = app.world_mut();
    let logical = &world.resource::<MapEvents>().events[0];
    assert_eq!(
        (logical.x, logical.y),
        (5, 6),
        "the routed NPC stepped down and its logical tile followed",
    );
    let (sprite, queue) = world
        .query::<(&EventSprite, &MoveQueue)>()
        .single(world)
        .unwrap();
    assert_eq!(sprite.dir, DIR_DOWN, "faced its move direction");
    assert!(queue.busy(), "the tile step is queued for the walk tween");
}

#[test]
fn forced_hero_route_advances_while_an_event_runs() {
    // The intro's forced route must keep moving while its foreground script waits.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(MapData::for_test(10, 10));
    app.insert_resource(MapEvents { events: Vec::new() });
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
    app.insert_resource(TitleActive(false));
    app.insert_resource(running_event());
    app.add_message::<AudioRequest>();
    app.add_systems(Update, route_hero);
    app.world_mut().spawn((
        Player {
            tile_x: 5,
            tile_y: 5,
            dir: DIR_DOWN,
            frame: 1,
            charset: "C".into(),
            index: 0,
        },
        MoveQueue::default(),
        RouteStepper::from_move_event(&[10001, 6, 0, 0, 2]),
        Sprite::default(),
    ));
    app.update();
    let world = app.world_mut();
    let queue = world.query::<&MoveQueue>().single(world).unwrap();
    assert!(
        queue.busy(),
        "a forced hero route must step during a running event (cutscene movement)",
    );
}

#[test]
fn page_route_pauses_while_an_event_runs() {
    // A page's own route does not bypass the foreground-event pause.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(MapData::for_test(10, 10));
    app.insert_resource(MapEvents {
        events: vec![Event {
            id: 1,
            x: 5,
            y: 5,
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
    app.insert_resource(TitleActive(false));
    app.insert_resource(running_event());
    app.add_message::<AudioRequest>();
    app.add_systems(Update, route_events);
    app.world_mut().spawn(Player {
        tile_x: 0,
        tile_y: 0,
        dir: DIR_DOWN,
        frame: 1,
        charset: "C".into(),
        index: 0,
    });
    let route = MoveRouteDef {
        commands: vec![MoveCommandDef {
            code: 2,
            params: Vec::new(),
            string: String::new(),
        }],
        repeat: true,
        skippable: false,
    };
    app.world_mut().spawn((
        EventSprite {
            id: 1,
            tile_x: 5,
            tile_y: 5,
            dir: DIR_DOWN,
            frame: 1,
            charset: "C".into(),
            index: 0,
            layer: 1,
        },
        MoveQueue::default(),
        RouteStepper::from_page(&route, 6, 8),
        Sprite::default(),
    ));
    app.update();
    let world = app.world_mut();
    let logical = &world.resource::<MapEvents>().events[0];
    assert_eq!(
        (logical.x, logical.y),
        (5, 5),
        "a page route must not step while an event runs",
    );
}
