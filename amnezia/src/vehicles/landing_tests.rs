use super::*;
use crate::menu::MenuOpen;
use amnezia_data::Map;

#[derive(Resource, Default)]
struct AudioLog(Vec<AudioRequest>);

fn collect_audio(mut messages: MessageReader<AudioRequest>, mut log: ResMut<AudioLog>) {
    log.0.extend(messages.read().cloned());
}

#[test]
fn landing_keyboard_blocks_every_active_event_layer_and_releases_after_page_removal() {
    for layer in 0..=2 {
        let mut map = load_ron::<Map>(&format!("{}/maps/map_0094.ron", asset_root()));
        let mut event = map.events.remove(0);
        event.x = 4;
        event.y = 4;
        event.pages.truncate(1);
        event.pages[0].layer = layer;
        event.pages[0].condition = default();
        let (mut app, hero) = test_support::rider_app(MapData::for_test(10, 10), (4, 4));
        app.insert_resource(MapEvents {
            events: vec![event],
        })
        .init_resource::<AudioLog>()
        .add_systems(Update, collect_audio.after(VehicleStep));
        let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
        vehicles.set_location(2, 0, 4, 4);
        vehicles.save.riding = Some(2);
        vehicles.save.before_music = Some(crate::audio::BgmTrack {
            name: "Field".into(),
            volume: 0.7,
            speed: 1.0,
            fade_in: 0.0,
        });
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = default();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        app.world_mut()
            .get_mut::<RouteStepper>(hero)
            .unwrap()
            .force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 32, 8, 1]));
        app.world_mut().resource_mut::<MenuOpen>().0 = true;
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(app.world().resource::<Vehicles>().airship_altitude(), 16.0);
        app.world_mut().resource_mut::<MenuOpen>().0 = false;
        for _ in 0..32 {
            app.update();
        }
        assert!(app.world().resource::<Vehicles>().riding(), "layer {layer}");
        assert_eq!(app.world().resource::<Vehicles>().airship_altitude(), 0.0);
        assert_eq!(
            app.world().resource::<Vehicles>().character(10004),
            Some((4, 4, DIR_LEFT))
        );
        assert!(!app.world().resource::<Switches>().get(8));
        assert!(app.world().resource::<AudioLog>().0.is_empty());
        app.world_mut().resource_mut::<MapEvents>().events[0]
            .pages
            .clear();
        for _ in 0..32 {
            app.update();
        }
        // A keyboard decision requires a stopped hero without a forced route.
        *app.world_mut().get_mut::<RouteStepper>(hero).unwrap() = default();
        *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = default();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert!(app.world().resource::<Vehicles>().airship_transitioning());
        app.world_mut()
            .get_mut::<RouteStepper>(hero)
            .unwrap()
            .force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 32, 8, 1]));
        *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = default();
        for _ in 0..31 {
            app.update();
        }
        assert!(app.world().resource::<AudioLog>().0.is_empty());
        assert!(app.world().resource::<Vehicles>().riding());
        app.update();
        assert!(!app.world().resource::<Vehicles>().riding());
        assert!(!app.world().resource::<Switches>().get(8));
        assert!(
            matches!(app.world().resource::<AudioLog>().0.as_slice(), [AudioRequest::Bgm { name, .. }] if name == "Field")
        );
        app.update();
        assert_eq!(app.world().resource::<AudioLog>().0.len(), 1);
        assert!(app.world().resource::<Switches>().get(8));
    }
}

#[test]
fn airship_cannot_land_on_a_boat_or_ship_even_without_an_event() {
    let data = MapData::for_test(10, 10);
    for index in 0..=1 {
        let mut vehicles = Vehicles::default();
        vehicles.set_location(2, 0, 4, 4);
        vehicles.set_location(index, 0, 4, 4);
        vehicles.save.riding = Some(2);
        vehicles.save.airship_flight.descend();
        for _ in 0..32 {
            vehicles.advance_flight(1.0 / 60.0, &data, |_, _| false);
        }
        assert!(vehicles.riding());
        vehicles.set_location(index, 1, 4, 4);
        for _ in 0..32 {
            vehicles.advance_flight(1.0 / 60.0, &data, |_, _| false);
        }
        vehicles.save.airship_flight.descend();
        for _ in 0..32 {
            vehicles.advance_flight(1.0 / 60.0, &data, |_, _| false);
        }
        assert!(!vehicles.riding());
    }
}
