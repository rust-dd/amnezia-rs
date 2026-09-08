use super::*;
use super::{clamp_to_map, ease_toward};
use amnezia_data::{Event, EventCommand, Map};
use bevy::math::Vec2;

fn event(layer: u32, trigger: u32) -> Event {
    let mut page = crate::assets::load_ron::<Map>(&format!(
        "{}/maps/map_0001.ron",
        crate::assets::asset_root()
    ))
    .events[0]
        .pages[0]
        .clone();
    page.layer = layer;
    page.trigger = trigger;
    page.condition = default();
    page.commands = vec![EventCommand {
        code: 11410,
        indent: 0,
        string: String::new(),
        params: vec![100],
    }];
    Event {
        id: 1,
        x: 4,
        y: 3,
        name: String::new(),
        pages: vec![page],
    }
}

fn movement_app(events: Vec<Event>) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .insert_resource(MapData::for_test(10, 10))
        .insert_resource(MapEvents { events })
        .init_resource::<Switches>()
        .init_resource::<Variables>()
        .init_resource::<Party>()
        .init_resource::<Inventory>()
        .init_resource::<Dialogue>()
        .init_resource::<RunningEvent>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (move_player, walk::<Player>).chain());
    app.world_mut().spawn((
        Player {
            tile_x: 3,
            tile_y: 3,
            dir: DIR_DOWN,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
        },
        MoveQueue::default(),
        RouteStepper::default(),
        Sprite::default(),
        Transform::default(),
    ));
    app.update();
    app
}

#[test]
fn floor_touch_waits_for_arrival_and_does_not_fire_again_at_rest() {
    let mut app = movement_app(vec![event(0, 1)]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    assert!(!app.world().resource::<RunningEvent>().active());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ArrowRight);
    for _ in 0..15 {
        app.update();
    }
    assert!(app.world().resource::<RunningEvent>().active());
    *app.world_mut().resource_mut::<RunningEvent>() = default();
    app.update();
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn floor_touch_cannot_be_activated_through_a_blocking_event() {
    let mut blocker = event(1, 0);
    blocker.id = 2;
    let mut app = movement_app(vec![event(0, 1), blocker]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    for _ in 0..20 {
        app.update();
    }
    assert!(!app.world().resource::<RunningEvent>().active());
    let world = app.world_mut();
    assert_eq!(world.query::<&Player>().single(world).unwrap().tile_x, 3);
}

#[test]
fn menu_freezes_a_step_already_in_flight() {
    let mut app = movement_app(vec![]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ArrowRight);
    let world = app.world_mut();
    let before = world
        .query::<&Transform>()
        .single(world)
        .unwrap()
        .translation;
    app.insert_resource(crate::menu::MenuOpen(true));
    for _ in 0..30 {
        app.update();
    }
    let world = app.world_mut();
    assert_eq!(
        world
            .query::<&Transform>()
            .single(world)
            .unwrap()
            .translation,
        before
    );
    assert!(world.query::<&MoveQueue>().single(world).unwrap().busy());
}

#[test]
fn pan_eases_toward_target_then_snaps() {
    // A full step moves exactly `step` along the direction to the target.
    assert_eq!(
        ease_toward(Vec2::ZERO, Vec2::new(10.0, 0.0), 2.0),
        Vec2::new(2.0, 0.0)
    );
    // Within one step of the target: snap onto it, no overshoot.
    assert_eq!(
        ease_toward(Vec2::new(9.0, 0.0), Vec2::new(10.0, 0.0), 5.0),
        Vec2::new(10.0, 0.0)
    );
    // Already at the target: stay put (and don't normalise a zero delta).
    assert_eq!(
        ease_toward(Vec2::splat(4.0), Vec2::splat(4.0), 5.0),
        Vec2::splat(4.0)
    );
}

#[test]
fn camera_clamps_to_map_edges() {
    // map half-extent 320, viewport half 160: the camera stops at ±160
    assert_eq!(clamp_to_map(1000.0, 320.0, 160.0), 160.0);
    assert_eq!(clamp_to_map(-1000.0, 320.0, 160.0), -160.0);
    // well inside the map: follows the target exactly
    assert_eq!(clamp_to_map(50.0, 320.0, 160.0), 50.0);
    // map narrower than the viewport: centered, no gray edge
    assert_eq!(clamp_to_map(1000.0, 100.0, 160.0), 0.0);
}
