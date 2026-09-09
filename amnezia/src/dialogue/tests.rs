use super::*;

#[test]
fn action_key_reaches_an_event_on_the_opposite_loop_edge() {
    let mut page = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0001.ron",
        crate::assets::asset_root()
    ))
    .events[0]
        .pages[0]
        .clone();
    page.layer = 1;
    page.trigger = 0;
    page.condition = default();
    page.commands = vec![amnezia_data::EventCommand {
        code: 11410,
        indent: 0,
        string: String::new(),
        params: vec![100],
    }];
    let mut data = MapData::for_test(20, 30);
    data.scroll_type = 1;
    let mut app = App::new();
    app.insert_resource(data)
        .insert_resource(MapEvents {
            events: vec![amnezia_data::Event {
                id: 7,
                name: String::new(),
                x: 10,
                y: 29,
                pages: vec![page],
            }],
        })
        .init_resource::<Switches>()
        .init_resource::<Variables>()
        .init_resource::<Party>()
        .init_resource::<Inventory>()
        .init_resource::<Dialogue>()
        .init_resource::<RunningEvent>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, interact);
    app.world_mut().spawn(Player {
        tile_x: 10,
        tile_y: 0,
        dir: crate::tiles::DIR_UP,
        frame: 1,
        charset: String::new(),
        index: 0,
    });
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(7));
}
