use super::*;
use crate::world::test_support::gated;

fn observer() -> Event {
    let mut observer = page(vec![]);
    observer.trigger = 4;
    observer.commands = vec![EventCommand {
        params: vec![0, 1, 1, 0, 6, 10002, 1],
        ..counter()
    }];
    event(2, 9, vec![observer])
}

#[test]
fn vehicle_make_way_updates_through_and_other_layer_obstacles_before_collision() {
    for through in [false, true] {
        for layer in 0..=2 {
            let mut mover = gated(page(vec![command(1, 0)]));
            mover.layer = layer;
            let mut app = app(vec![event(1, 1, vec![mover]), observer()]);
            vehicle(&mut app, 0, 2, 1, if through { &[36] } else { &[] });
            app.update();
            app.world_mut().resource_mut::<Switches>().set(7, true);
            app.world_mut()
                .resource_mut::<Vehicles>()
                .set_route(10002, RouteStepper::from_move_event(&[10002, 8, 0, 0, 1]));
            assert_eq!(app.world().resource::<Vehicles>().route_through(0), through);
            app.update();
            assert_eq!(app.world().resource::<Variables>().get(1), 3);
            moved_once(&app, 0, (2, 1));
        }
    }
}

#[test]
fn vehicle_make_way_skips_callbacks_for_through_movers_and_ignores_other_maps() {
    for other_map in [false, true] {
        let mut app = app(vec![
            event(
                1,
                1,
                vec![page(if other_map {
                    vec![command(1, 0)]
                } else {
                    vec![command(36, 0), command(1, 0)]
                })],
            ),
            observer(),
        ]);
        vehicle(&mut app, 0, 2, 1, &[1]);
        if other_map {
            app.world_mut().resource_mut::<Vehicles>().save.vehicles[0]
                .definition
                .map_id = 99;
        }
        app.update();
        assert_eq!(app.world().resource::<Variables>().get(1), 2);
        assert_eq!(vehicle_tile(&app, 0), (if other_map { 2 } else { 3 }, 1));
    }
}

#[test]
fn vehicle_make_way_does_not_update_an_airship_for_a_walking_hero() {
    #[derive(Resource, Default)]
    struct Observed(i32);
    for hero in [false, true] {
        let mut npc = event(1, 5, vec![page(vec![command(1, 0)])]);
        npc.y = 5;
        let mut app = app(if hero { vec![] } else { vec![npc] });
        vehicle(&mut app, 2, 6, 5, &[1]);
        app.init_resource::<Observed>().add_systems(
            Update,
            (|vehicles: Res<Vehicles>, mut seen: ResMut<Observed>| {
                seen.0 = vehicles.save.vehicles[2].tile().0
            })
            .after(crate::player::PlayerStep)
            .before(VehicleStep),
        );
        if hero {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ArrowRight);
        }
        app.update();
        assert_eq!(
            app.world().resource::<Observed>().0,
            if hero { 6 } else { 7 }
        );
        assert_eq!(vehicle_tile(&app, 2), (7, 5));
        moved_once(&app, 2, (6, 5));
    }
}
