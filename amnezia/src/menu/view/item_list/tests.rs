use super::*;
use crate::menu::testkit;

#[test]
fn original_inventory_windows_use_a_thirty_two_pixel_help_and_two_column_list() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Startup, |mut commands: Commands| {
            commands
                .spawn(Node::default())
                .with_children(|panel| spawn(panel, &Handle::default()));
        });
    app.update();
    let world = app.world_mut();
    let root = world
        .query::<(Entity, &Part)>()
        .iter(world)
        .find(|(_, part)| matches!(part, Part::Root))
        .unwrap()
        .0;
    let children = world.get::<Children>(root).unwrap();
    let rects = children
        .iter()
        .map(|entity| {
            let node = world.get::<Node>(entity).unwrap();
            (node.left, node.top, node.width, node.height)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rects,
        [
            (Val::Px(0.0), Val::Px(0.0), Val::Px(960.0), Val::Px(96.0)),
            (Val::Px(0.0), Val::Px(96.0), Val::Px(960.0), Val::Px(624.0)),
        ]
    );
    let (_, cursor) = world
        .query::<(&Part, &Node)>()
        .iter(world)
        .find(|(part, _)| matches!(part, Part::Cursor))
        .unwrap();
    assert_eq!(
        (cursor.left, cursor.top, cursor.width, cursor.height),
        (Val::Px(12.0), Val::Px(24.0), Val::Px(456.0), Val::Px(48.0))
    );
    assert_eq!(world.query::<&PixelText>().iter(world).count(), 2);
    assert_eq!(world.query::<&Text>().iter(world).count(), 0);
}

#[test]
fn item_text_uses_original_count_separator_grid_and_disabled_palette() {
    let mut data = testkit::data();
    data.items.push(testkit::weapon(10, "Kard", 5));
    data.items.push(testkit::weapon(11, "Másik", 5));
    let mut inventory = Inventory::default();
    for (id, count) in [(testkit::ITEM_HERB, 1), (10, 12), (11, 99)] {
        inventory.add_item(id, count);
    }
    let text = entries(&data, &inventory);
    assert_eq!(text.size, UVec2::new(304, 192));
    assert_eq!(
        text.runs,
        [
            Run::new("Gyógyfű", 0, 2, DEFAULT),
            Run::new(":  1", 120, 2, DEFAULT),
            Run::new("Kard", 160, 2, DISABLED),
            Run::new(": 12", 280, 2, DISABLED),
            Run::new("Másik", 0, 18, DISABLED),
            Run::new(": 99", 120, 18, DISABLED),
        ]
    );
}

#[test]
fn description_uses_the_database_text_and_empty_inventory_adds_no_invented_labels() {
    let mut data = testkit::data();
    data.items[0].description = "Életerőt gyógyít".into();
    let mut inventory = Inventory::default();
    inventory.add_gold(123);
    assert!(entries(&data, &inventory).runs.is_empty());
    assert_eq!(help(&data, &inventory, 0).runs[0].text, "");
    inventory.add_item(testkit::ITEM_HERB, 1);
    assert_eq!(
        help(&data, &inventory, 0).runs,
        [Run::new("Életerőt gyógyít", 0, 2, DEFAULT)]
    );
    assert_eq!(help(&data, &inventory, 100).runs[0].text, "");
}

#[test]
fn every_original_item_retains_its_name_description_count_and_field_color() {
    let mut data = testkit::data();
    data.items = crate::assets::load_ron(&format!("{}/items.ron", crate::assets::asset_root()));
    let mut inventory = Inventory::default();
    for item in &data.items {
        inventory.add_item(item.id, 1);
    }
    let text = entries(&data, &inventory);
    assert_eq!(text.runs.len(), 404);
    for (index, item) in data.items.iter().enumerate() {
        let run = &text.runs[index * 2];
        assert_eq!(run.text, item.name);
        assert_eq!(
            run.color,
            if item.item_type == 6 {
                DEFAULT
            } else {
                DISABLED
            }
        );
        assert_eq!(
            help(&data, &inventory, index).runs[0].text,
            item.description
        );
    }
}
