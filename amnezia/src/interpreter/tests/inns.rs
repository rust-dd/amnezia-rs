use super::*;

#[test]
fn only_paid_parallel_inns_can_replace_an_occupied_message_window() {
    for cost in [0, 30] {
        for trigger in [3, 4] {
            let mut app = interp_app();
            app.world_mut().resource_mut::<Dialogue>().active = true;
            app.insert_resource(MapEvents {
                events: vec![map_event(1, trigger, vec![cmd(10730, 0, vec![1, cost, 1])])],
            });
            app.update();
            let count = app
                .world_mut()
                .resource_mut::<Messages<ShopRequest>>()
                .drain()
                .count();
            assert_eq!(
                count,
                usize::from(trigger == 4 && cost != 0),
                "trigger {trigger}, cost {cost}"
            );
        }
    }
}

#[test]
fn every_original_inn_request_keeps_its_second_wording_set_and_price() {
    let mut count = 0;
    for (map_id, expected) in [(2, 6), (52, 8), (119, 2)] {
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/maps/map_{map_id:04}.ron",
            crate::assets::asset_root()
        ));
        let commands = map
            .events
            .into_iter()
            .flat_map(|event| event.pages)
            .flat_map(|page| page.commands)
            .filter(|command| command.code == 10730)
            .collect::<Vec<_>>();
        assert_eq!(commands.len(), expected, "map {map_id}");
        for command in commands {
            assert_eq!(command.params[0], 1);
            assert!([30, 50, 200].contains(&command.params[1]));
            let cost = command.params[1];
            let mut app = interp_app();
            app.world_mut()
                .resource_mut::<RunningEvent>()
                .start(1, vec![command]);
            app.update();
            let requests = app
                .world_mut()
                .resource_mut::<Messages<ShopRequest>>()
                .drain()
                .collect::<Vec<_>>();
            assert!(
                matches!(requests.as_slice(), [ShopRequest::ShowInn { cost: actual, inn_type: 1, foreground: true }] if *actual == cost)
            );
            count += 1;
        }
    }
    assert_eq!(count, 16);
}
