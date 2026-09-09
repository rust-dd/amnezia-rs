use super::*;
use crate::assets::{asset_root, load_ron};

#[test]
fn fixed_graphic_pages_keep_their_pose_while_moving() {
    let mut fixed = page("Chara1", 0);
    fixed.animation_type = 4;
    fixed.pattern = 0;
    fixed.direction = 2;
    let mut app = app_with_event(Event {
        id: 1,
        x: 3,
        y: 4,
        name: String::new(),
        pages: vec![fixed],
    });
    app.add_systems(Update, crate::world::walk::<EventSprite>);
    let world = app.world_mut();
    world
        .query::<&mut MoveQueue>()
        .single_mut(world)
        .unwrap()
        .enqueue_route([crate::world::RouteAction::Step {
            dx: 1,
            dy: 0,
            face: 1,
        }]);
    app.update();
    let world = app.world_mut();
    let event = world.query::<&EventSprite>().single(world).unwrap();
    assert_eq!((event.frame, event.dir), (0, 2));
}

#[test]
fn idle_continuous_pages_update_the_displayed_sprite_at_the_original_rate() {
    let mut continuous = page("Chara1", 0);
    continuous.animation_type = 1;
    continuous.pattern = 1;
    continuous.direction = 2;
    continuous.move_speed = 4;
    let mut app = app_with_event(Event {
        id: 1,
        x: 3,
        y: 4,
        name: String::new(),
        pages: vec![continuous],
    });
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.add_systems(
        Update,
        crate::world::walk::<EventSprite>
            .after(refresh_pages)
            .before(super::super::update_event_sprites),
    );
    for _ in 0..8 {
        app.update();
    }
    let world = app.world_mut();
    let (event, sprite) = world
        .query::<(&EventSprite, &Sprite)>()
        .single(world)
        .unwrap();
    assert_eq!((event.tile_x, event.tile_y, event.frame), (3, 4, 2));
    assert_eq!(sprite.rect.unwrap().min, Vec2::new(48.0, 64.0));
}

#[test]
fn changing_to_a_fixed_page_during_movement_immediately_installs_its_direction_and_pose() {
    let mut fixed = page("Chara1", 1);
    fixed.animation_type = 4;
    fixed.pattern = 0;
    fixed.direction = 3;
    fixed.condition.flags = 1;
    fixed.condition.switch_a = 8;
    let mut app = app_with_event(Event {
        id: 1,
        x: 3,
        y: 4,
        name: String::new(),
        pages: vec![page("Chara1", 0), fixed],
    });
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.add_systems(
        Update,
        crate::world::walk::<EventSprite>
            .after(refresh_pages)
            .before(super::super::update_event_sprites),
    );
    let world = app.world_mut();
    world
        .query::<&mut MoveQueue>()
        .single_mut(world)
        .unwrap()
        .enqueue_route([crate::world::RouteAction::Step {
            dx: 1,
            dy: 0,
            face: 1,
        }]);
    app.update();
    app.world_mut().resource_mut::<Switches>().set(8, true);
    app.update();
    let world = app.world_mut();
    let (event, queue, sprite) = world
        .query::<(&EventSprite, &MoveQueue, &Sprite)>()
        .single(world)
        .unwrap();
    assert!(queue.busy());
    assert_eq!((event.frame, event.dir), (0, 3));
    assert_eq!(sprite.rect.unwrap().min, Vec2::new(72.0, 96.0));
}

fn page(graphic: &str, index: u32) -> EventPage {
    let mut page = load_ron::<amnezia_data::Map>(&format!("{}/maps/map_0001.ron", asset_root()))
        .events[0]
        .pages[0]
        .clone();
    page.graphic_name = graphic.into();
    page.graphic_index = index;
    page.condition = Default::default();
    page.move_type = 0;
    page.move_route = Default::default();
    page
}

fn app_with_event(event: Event) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<Switches>()
        .init_resource::<Variables>()
        .init_resource::<Party>()
        .init_resource::<Inventory>()
        .insert_resource(MapData::for_test(20, 15))
        .insert_resource(EventTileset(Handle::default()))
        .insert_resource(MapEvents {
            events: vec![event],
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (refresh_pages, super::super::update_event_sprites).chain(),
        );
    app.update();
    app
}

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
) {
    spawn_event(
        &mut commands,
        &server,
        (&switches, &variables, &party, &inventory),
        &events.events[0],
        (160.0, 120.0),
        &Handle::default(),
    );
}

#[test]
fn switching_pages_updates_graphics_and_keeps_the_events_live_position() {
    let mut hidden = page("", 0);
    hidden.condition.flags = 1;
    hidden.condition.switch_a = 8;
    let mut app = app_with_event(Event {
        id: 1,
        x: 3,
        y: 4,
        name: String::new(),
        pages: vec![page("Chara1", 0), hidden],
    });
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<EventSprite>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<EventSprite>(entity)
        .unwrap()
        .tile_x = 9;
    app.world_mut().resource_mut::<Switches>().set(8, true);
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(entity).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(app.world().get::<EventSprite>(entity).unwrap().tile_x, 9);
    app.world_mut().resource_mut::<Switches>().set(8, false);
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(entity).unwrap(),
        Visibility::Visible
    );
    assert_eq!(
        app.world().get::<EventSprite>(entity).unwrap().charset,
        "Chara1"
    );
}

#[test]
fn invisible_events_have_movement_components_and_tile_events_render_from_the_chipset() {
    let mut app = app_with_event(Event {
        id: 1,
        x: 3,
        y: 4,
        name: String::new(),
        pages: vec![page("", 80)],
    });
    let world = app.world_mut();
    let (sprite, _, _, transform) = world
        .query::<(&Sprite, &MoveQueue, &RouteStepper, &Transform)>()
        .single(world)
        .unwrap();
    assert_eq!(sprite.custom_size, Some(Vec2::splat(16.0)));
    assert_eq!(transform.translation.y, 48.0);
    let mut invisible = app_with_event(Event {
        id: 2,
        x: 1,
        y: 1,
        name: String::new(),
        pages: vec![],
    });
    let world = invisible.world_mut();
    assert_eq!(world.query::<&EventSprite>().iter(world).count(), 1);
    assert_eq!(world.query::<&RouteStepper>().iter(world).count(), 1);
}

#[test]
fn page_refresh_during_a_forced_route_installs_the_new_autonomous_program() {
    let mut next = page("Chara1", 1);
    next.condition.flags = 1;
    next.condition.switch_a = 8;
    next.move_type = 6;
    next.move_frequency = 8;
    next.move_speed = 5;
    next.move_route = amnezia_data::MoveRouteDef {
        commands: vec![
            amnezia_data::MoveCommandDef {
                code: 32,
                params: vec![9],
                string: String::new(),
            },
            amnezia_data::MoveCommandDef {
                code: 23,
                ..default()
            },
        ],
        repeat: true,
        skippable: false,
    };
    let mut app = app_with_event(Event {
        id: 1,
        x: 3,
        y: 4,
        name: String::new(),
        pages: vec![page("Chara1", 0), next],
    });
    let world = app.world_mut();
    world
        .query::<&mut RouteStepper>()
        .single_mut(world)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 23]));
    world.resource_mut::<Switches>().set(8, true);
    app.update();
    let world = app.world_mut();
    let (mut character, mut queue, mut route) = world
        .query::<(&mut EventSprite, &mut MoveQueue, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    assert!(route.forced());
    assert_eq!(route.speed(), 5);
    route.force_route(RouteStepper::from_move_event(&[1, 8, 0, 0]));
    assert!(!route.forced());
    assert!(route.active());
    let driven = crate::world::drive_route(
        &mut *character,
        &mut queue,
        &mut route,
        (0, 0),
        1.0 / 60.0,
        |_, _, _| true,
    );
    assert!(matches!(
        driven.effects.as_slice(),
        [crate::world::StepEffect::Switch(9, true)]
    ));
}
