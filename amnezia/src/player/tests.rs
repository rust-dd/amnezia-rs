use super::camera::{camera_follow, clamp_to_map, ease_toward};
use super::*;
use crate::world::MainCamera;
use amnezia_data::{Event, EventCommand, Map};
use bevy::math::Vec2;

mod logical;

#[test]
fn an_empty_party_graphic_stays_hidden_until_an_actor_returns() {
    let mut app = App::new();
    app.init_resource::<HeroHidden>()
        .add_systems(Update, update_hero_hidden);
    let hero = app
        .world_mut()
        .spawn((
            Player {
                tile_x: 0,
                tile_y: 0,
                dir: 0,
                frame: 1,
                charset: String::new(),
                index: 0,
            },
            Visibility::Inherited,
        ))
        .id();
    for (charset, scripted, expected) in [
        ("", false, Visibility::Hidden),
        ("Chara1", true, Visibility::Hidden),
        ("Chara1", false, Visibility::Inherited),
    ] {
        app.world_mut().get_mut::<Player>(hero).unwrap().charset = charset.into();
        app.world_mut().resource_mut::<HeroHidden>().0 = scripted;
        app.update();
        assert_eq!(*app.world().get::<Visibility>(hero).unwrap(), expected);
    }
}

#[test]
fn parallel_choice_and_number_prompts_block_keyboard_movement() {
    for numeric in [false, true] {
        let mut app = movement_app(vec![]);
        if numeric {
            let mut input = crate::inputnumber::InputNumber::default();
            input.open(3, 1);
            app.insert_resource(input);
        } else {
            let mut choice = crate::choice::Choice::default();
            choice.open(vec!["Igen".into()], 0, 0);
            app.insert_resource(choice);
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        app.update();
        let world = app.world_mut();
        let hero = world.query::<&Player>().single(world).unwrap();
        assert_eq!((hero.tile_x, hero.tile_y), (3, 3));
    }
}

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
        .add_systems(
            Update,
            (move_player, walk::<Player>).chain().in_set(PlayerStep),
        );
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
fn keyboard_movement_uses_the_other_events_live_through_state() {
    let other = event(1, 0);
    let mut app = movement_app(vec![other]);
    let mut character = EventSprite {
        id: 1,
        tile_x: 4,
        tile_y: 3,
        dir: DIR_DOWN,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
        layer: 1,
    };
    let mut route = RouteStepper::from_move_event(&[1, 8, 0, 0, 36]);
    crate::world::drive_route(
        &mut character,
        &mut MoveQueue::default(),
        &mut route,
        (0, 0),
        1.0 / 60.0,
        |_, _, _, _, _| true,
    );
    app.world_mut().spawn((character, route));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    let world = app.world_mut();
    assert_eq!(world.query::<&Player>().single(world).unwrap().tile_x, 4);
    assert!(!world.resource::<RunningEvent>().active());
}

#[test]
fn keyboard_movement_keeps_scripted_speed_through_and_facing_lock() {
    let mut app = movement_app(vec![event(1, 0)]);
    let world = app.world_mut();
    let (mut player, mut queue, mut route) = world
        .query::<(&mut Player, &mut MoveQueue, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    route.force_route(RouteStepper::from_move_event(&[
        10001, 8, 0, 0, 36, 26, 29, 29,
    ]));
    crate::world::drive_route(
        &mut *player,
        &mut queue,
        &mut route,
        (0, 0),
        1.0 / 60.0,
        |_, _, _, _, through| through,
    );
    assert!(!route.forced());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ArrowRight);
    for _ in 0..10 {
        app.update();
    }
    let world = app.world_mut();
    assert!(world.query::<&MoveQueue>().single(world).unwrap().busy());
    for _ in 0..30 {
        app.update();
    }
    let world = app.world_mut();
    let player = world.query::<&Player>().single(world).unwrap();
    assert_eq!((player.tile_x, player.tile_y, player.dir), (4, 3, DIR_DOWN));
    assert!(!world.resource::<RunningEvent>().active());
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
fn continue_events_does_not_restore_player_input_during_dialogue() {
    let mut app = movement_app(vec![]);
    app.insert_resource(crate::dialogue::MessageOptions {
        fixed: false,
        continue_events: true,
    });
    app.world_mut().resource_mut::<Dialogue>().active = true;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    for _ in 0..20 {
        app.update();
    }
    let world = app.world_mut();
    assert_eq!(world.query::<&Player>().single(world).unwrap().tile_x, 3);
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

#[test]
fn keyboard_wraps_and_triggers_events_across_the_seam() {
    for (looping, layer) in [(false, 0), (true, 0), (true, 1)] {
        let mut destination = event(layer, 1);
        destination.x = 9;
        let mut app = movement_app(vec![destination]);
        app.world_mut().resource_mut::<MapData>().scroll_type = if looping { 2 } else { 0 };
        let world = app.world_mut();
        world
            .query::<&mut Player>()
            .single_mut(world)
            .unwrap()
            .tile_x = 0;
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowLeft);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ArrowLeft);
        for _ in 0..15 {
            app.update();
        }
        let world = app.world_mut();
        let player = world.query::<&Player>().single(world).unwrap();
        assert_eq!(player.tile_x, if looping && layer == 0 { 9 } else { 0 });
        assert_eq!(world.resource::<RunningEvent>().active(), looping);
    }
}

#[test]
fn camera_only_clamps_non_looping_axes() {
    for mode in 0..=3 {
        let mut data = MapData::for_test(140, 140);
        data.scroll_type = mode;
        let mut app = App::new();
        app.init_resource::<Time>()
            .insert_resource(data)
            .init_resource::<CameraPan>()
            .add_systems(Update, camera_follow);
        app.world_mut().spawn((
            Player {
                tile_x: 0,
                tile_y: 0,
                dir: 0,
                frame: 1,
                charset: String::new(),
                index: 0,
            },
            Transform::from_xyz(-1120.0, 1120.0, 4.0),
        ));
        let camera = app
            .world_mut()
            .spawn((
                MainCamera,
                Transform::default(),
                Projection::Orthographic(OrthographicProjection {
                    area: Rect::new(-160.0, -120.0, 160.0, 120.0),
                    ..OrthographicProjection::default_2d()
                }),
            ))
            .id();
        app.update();
        let position = app.world().get::<Transform>(camera).unwrap().translation;
        assert_eq!(position.x, if mode & 2 != 0 { -1112.0 } else { -960.0 });
        assert_eq!(position.y, if mode & 1 != 0 { 1112.0 } else { 1000.0 });
    }
}
