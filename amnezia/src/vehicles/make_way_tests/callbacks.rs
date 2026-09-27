use super::*;
use crate::world::test_support::gated;

fn script(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        params,
        ..counter()
    }
}

fn callback(mut commands: Vec<EventCommand>) -> Event {
    commands.push(script(10210, vec![0, 7, 7, 0]));
    let mut parallel = page(vec![]);
    parallel.trigger = 4;
    parallel.commands = commands;
    event(2, 3, vec![parallel, gated(page(vec![]))])
}

#[test]
fn vehicle_make_way_observes_a_replaced_forced_routes_cursor_and_skip_flag() {
    let mut app = app(vec![
        event(1, 1, vec![page(vec![command(1, 0)])]),
        callback(vec![script(11330, vec![10002, 8, 0, 1, 23, 32, 8])]),
    ]);
    vehicle(&mut app, 0, 2, 1, &[1, 32, 9]);
    app.update();
    assert!(app.world().resource::<Switches>().get(8));
    assert!(!app.world().resource::<Switches>().get(9));
    assert_eq!(vehicle_tile(&app, 0), (2, 1));
}

#[test]
fn vehicle_make_way_cancellation_keeps_the_attempt_but_does_not_replay_its_tail() {
    for jumping in [false, true] {
        let mut app = app(vec![
            event(1, 1, vec![page(vec![command(1, 0)])]),
            callback(vec![
                script(11330, vec![10002, 8, 0, 0]),
                script(10860, vec![2, 0, 9, 1]),
            ]),
        ]);
        vehicle(
            &mut app,
            0,
            2,
            1,
            if jumping {
                &[24, 1, 25, 32, 9]
            } else {
                &[1, 32, 9]
            },
        );
        app.update();
        assert_eq!(vehicle_tile(&app, 0), (3, 1));
        let vehicles = app.world().resource::<Vehicles>();
        assert!(!vehicles.motion[0].route.forced());
        assert_eq!(vehicles.jumping(0), jumping);
        let data = app.world().resource::<MapData>();
        let ground = vehicles.motion[0]
            .queue
            .ground_position(&vehicles.save.vehicles[0], data);
        assert_eq!(
            ground,
            Vec2::from(data.tile_center(2, 1)) + Vec2::X * if jumping { 1.0 } else { 2.0 }
        );
        for _ in 0..15 {
            app.update();
        }
        assert!(!app.world().resource::<Switches>().get(9));
    }
}

#[test]
fn vehicle_make_way_airship_movers_skip_character_callbacks() {
    let mut target = page(vec![]);
    target.trigger = 4;
    target.commands = vec![counter()];
    let mut app = app(vec![
        event(1, 1, vec![page(vec![command(1, 0)])]),
        event(2, 3, vec![target]),
    ]);
    vehicle(&mut app, 2, 2, 1, &[1]);
    app.update();
    assert_eq!(vehicle_tile(&app, 2), (3, 1));
    assert_eq!(app.world().resource::<Variables>().get(1), 1);
    moved_once(&app, 2, (2, 1));
}

#[test]
fn vehicle_make_way_does_not_recheck_an_earlier_vehicle_after_a_later_callback() {
    let mut app = app(vec![
        event(1, 1, vec![page(vec![command(1, 0)])]),
        callback(vec![
            script(10850, vec![0, 0, 0, 2, 1]),
            script(10860, vec![2, 0, 9, 1]),
        ]),
    ]);
    vehicle(&mut app, 0, 8, 1, &[]);
    vehicle(&mut app, 1, 2, 1, &[1]);
    app.update();
    let npc = entity(&mut app, 1);
    assert_eq!(
        app.world()
            .get::<crate::world::EventSprite>(npc)
            .unwrap()
            .tile(),
        (2, 1)
    );
    assert_eq!(vehicle_tile(&app, 0), (2, 1));
    assert_eq!(vehicle_tile(&app, 1), (3, 1));
    moved_once(&app, 1, (2, 1));
}
