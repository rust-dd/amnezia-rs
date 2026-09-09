use super::*;
use crate::world::{EventSprite, MainCamera};

#[test]
fn original_intro_pictures_read_this_events_live_screen_anchor_after_camera_movement() {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0029.ron",
        crate::assets::asset_root()
    ));
    for (event_id, name, tile, expected) in [
        (1, "Intro1", (5, 12), (64.0, 216.0)),
        (2, "Intro2", (16, 2), (240.0, 56.0)),
    ] {
        let event = map.events.iter().find(|e| e.id == event_id).unwrap();
        let commands = &event.pages[0].commands;
        let start = commands
            .iter()
            .position(|c| c.code == 10220 && c.params[4] == 6)
            .unwrap();
        let mut app = interp_app();
        let mut data = MapData::for_test(map.width as i32, map.height as i32);
        data.map_id = 29;
        let (wx, wy) = data.tile_center(tile.0, tile.1);
        app.insert_resource(data);
        app.world_mut()
            .spawn((MainCamera, Transform::from_xyz(-144.0, 8.0, 0.0)));
        app.world_mut().spawn((
            EventSprite {
                id: event_id,
                tile_x: tile.0,
                tile_y: tile.1,
                dir: 2,
                frame: 1,
                charset: String::new(),
                index: 0,
                layer: 1,
            },
            MoveQueue::default(),
            RouteStepper::default(),
            Transform::from_xyz(wx - 8.0, wy, 4.0),
        ));
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(event_id, commands[start..start + 3].to_vec());
        app.update();
        assert_eq!(
            app.world().resource::<Variables>().get(12),
            expected.0 as i32
        );
        assert_eq!(
            app.world().resource::<Variables>().get(13),
            expected.1 as i32
        );
        let pictures = app.world().resource::<Messages<PictureCommand>>();
        let mut cursor = pictures.get_cursor();
        assert!(cursor.read(pictures).any(|p| matches!(p, PictureCommand::Show { name: picture, x, y, .. } if picture == name && (*x, *y) == expected)));
    }
}

#[test]
fn hero_vehicle_and_event_map_ids_keep_the_rpg2000_distinction() {
    let mut app = interp_app();
    app.world_mut().resource_mut::<MapData>().map_id = 13;
    app.world_mut()
        .resource_mut::<crate::vehicles::Vehicles>()
        .set_location(2, 96, 10, 11);
    app.world_mut().spawn((
        EventSprite {
            id: 7,
            tile_x: 4,
            tile_y: 3,
            dir: 2,
            frame: 1,
            charset: String::new(),
            index: 0,
            layer: 1,
        },
        MoveQueue::default(),
        RouteStepper::default(),
    ));
    app.world_mut().resource_mut::<RunningEvent>().start(
        7,
        vec![
            cmd(10220, 0, vec![0, 1, 1, 0, 6, 10001, 0]),
            cmd(10220, 0, vec![0, 2, 2, 0, 6, 10004, 0]),
            cmd(10220, 0, vec![0, 3, 3, 0, 6, 10005, 0]),
            cmd(10220, 0, vec![0, 4, 4, 0, 6, 9999, 4]),
        ],
    );
    app.update();
    let vars = app.world().resource::<Variables>();
    assert_eq!(
        (vars.get(1), vars.get(2), vars.get(3), vars.get(4)),
        (13, 96, 0, 0)
    );
}
