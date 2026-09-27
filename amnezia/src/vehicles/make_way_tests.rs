use super::*;
use crate::world::test_support::{command, entity, event, page, water_map};
use amnezia_data::{Event, EventCommand};

mod callbacks;
mod logical;
mod ordering;

fn app(events: Vec<Event>) -> App {
    let mut app = crate::world::test_support::app(events, false);
    app.insert_resource(water_map(20, 20))
        .init_resource::<CurrentBgm>()
        .init_resource::<crate::system_bgm::SystemBgm>()
        .add_plugins(VehiclePlugin);
    for index in 0..3 {
        app.world_mut()
            .resource_mut::<Vehicles>()
            .set_location(index, 99, 0, 0);
    }
    app
}

fn vehicle(app: &mut App, index: usize, x: u32, y: u32, commands: &[i32]) {
    let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
    vehicles.set_location(index, 0, x, y);
    vehicles.save.vehicles[index].speed = 4;
    let reference = 10002 + index as i32;
    let mut params = vec![reference, 8, 0, 0];
    params.extend_from_slice(commands);
    vehicles.set_route(reference, RouteStepper::from_move_event(&params));
}

fn counter() -> EventCommand {
    EventCommand {
        code: 10220,
        indent: 0,
        string: String::new(),
        params: vec![0, 1, 1, 1, 0, 1],
    }
}

fn vehicle_tile(app: &App, index: usize) -> (i32, i32) {
    app.world().resource::<Vehicles>().save.vehicles[index].tile()
}

fn moved_once(app: &App, index: usize, origin: (i32, i32)) {
    let data = app.world().resource::<MapData>();
    let vehicles = app.world().resource::<Vehicles>();
    assert_eq!(
        vehicles.motion[index].pixel.unwrap(),
        Vec2::from(data.tile_center(origin.0, origin.1)) + Vec2::X * 2.0
    );
    assert_eq!(vehicles.motion[index].route.stop_count(), 0);
}

#[test]
fn vehicle_make_way_precedes_later_parallel_queries_and_moves_only_once() {
    let mut observer = page(vec![]);
    observer.trigger = 4;
    observer.commands = vec![EventCommand {
        params: vec![0, 1, 1, 0, 6, 10002, 1],
        ..counter()
    }];
    let mut app = app(vec![
        event(1, 1, vec![page(vec![command(1, 0)])]),
        event(2, 9, vec![observer]),
    ]);
    vehicle(&mut app, 0, 2, 1, &[1]);
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(1), 3);
    let npc = entity(&mut app, 1);
    assert_eq!(
        app.world()
            .get::<crate::world::EventSprite>(npc)
            .unwrap()
            .tile(),
        (2, 1)
    );
    assert_eq!(vehicle_tile(&app, 0), (3, 1));
    moved_once(&app, 0, (2, 1));
}

#[test]
fn vehicle_make_way_extends_npc_chains_and_revisits_their_parallel_scripts() {
    let mut target = page(vec![command(1, 0)]);
    target.trigger = 4;
    target.commands = vec![counter()];
    let mut app = app(vec![
        event(1, 1, vec![page(vec![command(1, 0)])]),
        event(2, 3, vec![target]),
    ]);
    vehicle(&mut app, 0, 2, 1, &[1]);
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(1), 2);
    for (id, x) in [(1, 2), (2, 4)] {
        let npc = entity(&mut app, id);
        assert_eq!(
            app.world()
                .get::<crate::world::EventSprite>(npc)
                .unwrap()
                .tile(),
            (x, 1)
        );
    }
    assert_eq!(vehicle_tile(&app, 0), (3, 1));
    moved_once(&app, 0, (2, 1));
}

#[test]
fn vehicle_make_way_visits_boat_ship_and_parked_airship_in_one_update() {
    let mut app = app(vec![event(1, 1, vec![page(vec![command(1, 0)])])]);
    for index in 0..3 {
        vehicle(&mut app, index, 2 + index as u32, 1, &[1]);
    }
    app.update();
    for index in 0..3 {
        assert_eq!(vehicle_tile(&app, index), (3 + index as i32, 1));
        moved_once(&app, index, (2 + index as i32, 1));
    }
    let npc = entity(&mut app, 1);
    assert_eq!(
        app.world()
            .get::<crate::world::EventSprite>(npc)
            .unwrap()
            .tile(),
        (2, 1)
    );
}

#[test]
fn vehicle_make_way_cycles_cannot_reenter_a_processed_character() {
    let mut app = app(vec![]);
    vehicle(&mut app, 0, 2, 1, &[1]);
    vehicle(&mut app, 1, 3, 1, &[3]);
    let before = std::array::from_fn::<_, 2, _>(|index| {
        app.world().resource::<Vehicles>().motion[index]
            .route
            .stop_count()
    });
    app.update();
    for (index, before) in before.into_iter().enumerate() {
        assert_eq!(vehicle_tile(&app, index), (2 + index as i32, 1));
        assert_eq!(
            app.world().resource::<Vehicles>().motion[index]
                .route
                .stop_count(),
            before + 1
        );
    }
}

#[test]
fn vehicle_make_way_works_for_manual_and_forced_walking_heroes() {
    for forced in [false, true] {
        let mut app = app(vec![]);
        vehicle(&mut app, 0, 6, 5, &[1]);
        let hero = app
            .world_mut()
            .query_filtered::<Entity, With<Player>>()
            .single(app.world())
            .unwrap();
        if forced {
            app.world_mut()
                .get_mut::<RouteStepper>(hero)
                .unwrap()
                .force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 1]));
        } else {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ArrowRight);
        }
        app.update();
        assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (6, 5));
        assert_eq!(vehicle_tile(&app, 0), (7, 5));
        moved_once(&app, 0, (6, 5));
    }
}

#[test]
fn vehicle_make_way_can_update_the_hero_inside_an_earlier_npc_chain() {
    let mut mover = event(1, 3, vec![page(vec![command(1, 0)])]);
    mover.y = 5;
    let mut observer = page(vec![]);
    observer.trigger = 4;
    observer.commands = vec![EventCommand {
        params: vec![0, 1, 1, 0, 6, 10001, 1],
        ..counter()
    }];
    let mut app = app(vec![mover, event(2, 9, vec![observer])]);
    vehicle(&mut app, 0, 4, 5, &[1]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(1), 6);
    assert_eq!(vehicle_tile(&app, 0), (5, 5));
    moved_once(&app, 0, (4, 5));
    let npc = entity(&mut app, 1);
    assert_eq!(
        app.world()
            .get::<crate::world::EventSprite>(npc)
            .unwrap()
            .tile(),
        (4, 5)
    );
}
