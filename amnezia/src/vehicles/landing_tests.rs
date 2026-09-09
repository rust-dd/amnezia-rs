use super::*;
use crate::battle::BattleActive;
use crate::dialogue::Dialogue;
use crate::gameover::GameOverActive;
use crate::interpreter::RunningEvent;
use crate::menu::MenuOpen;
use crate::shop::ShopOpen;
use crate::teleport::Fade;
use crate::title::TitleActive;
use amnezia_data::Map;

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
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(MapData::for_test(10, 10))
            .insert_resource(MapEvents {
                events: vec![event],
            })
            .init_resource::<Switches>()
            .init_resource::<Variables>()
            .init_resource::<Party>()
            .init_resource::<Inventory>()
            .init_resource::<Dialogue>()
            .init_resource::<Fade>()
            .init_resource::<MenuOpen>()
            .init_resource::<ShopOpen>()
            .init_resource::<BattleActive>()
            .init_resource::<GameOverActive>()
            .init_resource::<RunningEvent>()
            .insert_resource(TitleActive(false))
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<CurrentBgm>()
            .init_resource::<crate::system_bgm::SystemBgm>()
            .insert_resource(VehicleMusic(std::array::from_fn(|_| default())))
            .init_resource::<Vehicles>()
            .add_message::<AudioRequest>()
            .add_systems(Update, keyboard);
        app.world_mut().spawn((
            Player {
                tile_x: 4,
                tile_y: 4,
                dir: DIR_DOWN,
                frame: 1,
                charset: "Chara1".into(),
                index: 0,
            },
            MoveQueue::default(),
        ));
        let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
        vehicles.set_location(2, 0, 4, 4);
        vehicles.save.riding = Some(2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert!(app.world().resource::<Vehicles>().riding(), "layer {layer}");
        app.world_mut().resource_mut::<MapEvents>().events[0]
            .pages
            .clear();
        app.update();
        assert!(!app.world().resource::<Vehicles>().riding());
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
        assert!(!vehicles.toggle(&data, (4, 4, DIR_DOWN), |_, _| false));
        vehicles.set_location(index, 1, 4, 4);
        assert!(vehicles.toggle(&data, (4, 4, DIR_DOWN), |_, _| false));
    }
}
