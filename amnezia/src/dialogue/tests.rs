use super::*;
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::state::{Inventory, Party, Switches, Variables};
use crate::world::{MapData, MapEvents};

#[test]
fn automatic_dialogue_position_is_chosen_once_per_box_and_battle_stays_at_the_bottom() {
    let mut app = App::new();
    app.insert_resource(MapData::for_test(20, 15))
        .init_resource::<MessagePosition>()
        .init_resource::<MessageOptions>()
        .init_resource::<Dialogue>()
        .add_systems(Update, view::update_position);
    app.world_mut()
        .spawn((crate::world::MainCamera, Transform::default()));
    let hero = app
        .world_mut()
        .spawn((
            Player {
                tile_x: 10,
                tile_y: 11,
                dir: 2,
                frame: 1,
                charset: String::new(),
                index: 0,
            },
            Transform::from_xyz(0.0, -56.0, 4.0),
        ))
        .id();
    let panel = app
        .world_mut()
        .spawn((view::DialoguePanel, Node::default()))
        .id();
    app.world_mut().resource_mut::<Dialogue>().open(vec![
        MessageBox {
            face: None,
            face_index: 0,
            lines: vec!["Első".into()],
        },
        MessageBox {
            face: None,
            face_index: 0,
            lines: vec!["Második".into()],
        },
    ]);
    app.update();
    assert_eq!(app.world().get::<Node>(panel).unwrap().top, Val::Px(0.0));
    app.world_mut()
        .get_mut::<Transform>(hero)
        .unwrap()
        .translation
        .y = 72.0;
    app.update();
    assert_eq!(app.world().get::<Node>(panel).unwrap().top, Val::Px(0.0));
    app.world_mut().resource_mut::<Dialogue>().advance(0);
    app.update();
    assert_eq!(app.world().get::<Node>(panel).unwrap().bottom, Val::Px(0.0));
    app.world_mut().resource_mut::<MessageOptions>().fixed = true;
    *app.world_mut().resource_mut::<MessagePosition>() = MessagePosition::Top;
    app.update();
    assert_eq!(app.world().get::<Node>(panel).unwrap().top, Val::Px(0.0));
    app.insert_resource(crate::battle::BattleActive(true));
    app.update();
    assert_eq!(app.world().get::<Node>(panel).unwrap().bottom, Val::Px(0.0));
}

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
    action::register(&mut app);
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
        .init_resource::<crate::timing::GameFrames>()
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
    let mut choice = crate::choice::Choice::default();
    choice.open(vec!["Igen".into()], 0, 0);
    app.insert_resource(choice);
    app.update();
    assert!(!app.world().resource::<RunningEvent>().active());
    app.world_mut().remove_resource::<crate::choice::Choice>();
    let mut number = crate::inputnumber::InputNumber::default();
    number.open(3, 1);
    app.insert_resource(number);
    app.update();
    assert!(!app.world().resource::<RunningEvent>().active());
    app.world_mut()
        .remove_resource::<crate::inputnumber::InputNumber>();
    app.update();
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(7));
}
