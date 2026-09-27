use super::*;
use crate::player::Player;
use amnezia_data::{EventPage, MoveCommandDef, MoveRouteDef};
use bevy::ecs::system::RunSystemOnce;

pub(crate) fn command(code: u32, parameter: i32) -> MoveCommandDef {
    MoveCommandDef {
        code,
        params: vec![parameter],
        ..default()
    }
}

pub(crate) fn water_map(width: i32, height: i32) -> MapData {
    let mut data = MapData::for_test(width, height);
    data.terrains[0].boat_pass = true;
    data.terrains[0].ship_pass = true;
    data.passages_up[0] |= crate::tiles::ABOVE_HERO_BIT;
    data
}

pub(crate) fn page(commands: Vec<MoveCommandDef>) -> EventPage {
    let mut page = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0001.ron",
        crate::assets::asset_root()
    ))
    .events[0]
        .pages[0]
        .clone();
    page.condition = default();
    page.graphic_name = "Chara1".into();
    page.graphic_index = 0;
    page.direction = 1;
    page.pattern = 1;
    page.animation_type = 0;
    page.layer = 1;
    page.translucent = false;
    page.trigger = 0;
    page.commands.clear();
    page.move_type = 6;
    page.move_frequency = 8;
    page.move_speed = 4;
    page.move_route = MoveRouteDef {
        commands,
        repeat: false,
        skippable: false,
    };
    page
}

pub(crate) fn gated(mut page: EventPage) -> EventPage {
    page.condition.flags = 1;
    page.condition.switch_a = 7;
    page
}

pub(crate) fn event(id: u32, x: u32, pages: Vec<EventPage>) -> Event {
    Event {
        id,
        x,
        y: 1,
        pages,
        name: String::new(),
    }
}

pub(crate) fn app(events: Vec<Event>, enabled: bool) -> App {
    let mut app = crate::interpreter::tests::interp_app();
    app.add_plugins((AssetPlugin::default(), crate::player::PlayerPlugin))
        .init_asset::<Image>()
        .insert_resource(pages::EventTileset(Handle::default()))
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
    crate::world::update::register(&mut app);
    let world = app.world_mut();
    let entity = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    world
        .entity_mut(entity)
        .insert((Sprite::default(), Transform::default()));
    app.update();
    app.insert_resource(MapEvents { events });
    app.world_mut().resource_mut::<Switches>().set(7, enabled);
    app.world_mut().run_system_once(spawn).unwrap();
    app
}

pub(crate) fn spawn(
    mut commands: Commands,
    server: Res<AssetServer>,
    events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
) {
    for event in &events.events {
        pages::spawn_event(
            &mut commands,
            &server,
            (&switches, &variables, &party, &inventory),
            event,
            (80.0, 80.0),
            &Handle::default(),
        );
    }
}

pub(crate) fn entity(app: &mut App, id: u32) -> Entity {
    let world = app.world_mut();
    world
        .query::<(Entity, &EventSprite)>()
        .iter(world)
        .find(|(_, character)| character.id == id)
        .unwrap()
        .0
}

pub(crate) fn camera_app(tile: (i32, i32)) -> (App, Entity) {
    let mut app = app(vec![], false);
    app.insert_resource(MapData::for_test(40, 30));
    let world = app.world_mut();
    let hero = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    world
        .get_mut::<Player>(hero)
        .unwrap()
        .set_tile(tile.0, tile.1);
    world.spawn((
        MainCamera,
        Transform::default(),
        Projection::Orthographic(OrthographicProjection {
            area: Rect::new(-160.0, -120.0, 160.0, 120.0),
            ..OrthographicProjection::default_2d()
        }),
    ));
    app.update();
    (app, hero)
}
