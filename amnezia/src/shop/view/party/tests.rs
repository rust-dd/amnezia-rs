use super::*;
use std::cmp::Ordering::{Equal, Greater, Less};

fn data() -> GameData {
    let mut app = App::new();
    app.add_plugins(crate::gamedata::GameDataPlugin);
    app.world_mut().remove_resource::<GameData>().unwrap()
}

#[test]
fn comparison_replaces_the_correct_slot_and_removes_the_other_hand_for_two_handed_gear() {
    let mut data = data();
    for (index, item_type, bonus, two_handed) in [
        (0, 1, 10, false),
        (1, 2, 50, false),
        (2, 1, 55, true),
        (3, 3, 5, false),
    ] {
        let item = &mut data.items[index];
        item.item_type = item_type;
        item.atk = bonus;
        item.def = 0;
        item.spi = 0;
        item.agi = 0;
        item.two_handed = two_handed;
    }
    for actor in &data.actors {
        for level in [1, 20] {
            assert_eq!(
                comparison(actor, level, &data, [1, 2, 0, 0, 0], data.item(3).unwrap()),
                Less
            );
            assert_eq!(
                comparison(actor, level, &data, [3, 0, 0, 0, 0], data.item(2).unwrap()),
                Less
            );
            assert_eq!(
                comparison(actor, level, &data, [1, 0, 0, 0, 0], data.item(3).unwrap()),
                Greater
            );
            assert_eq!(
                comparison(actor, level, &data, [1, 2, 4, 0, 0], data.item(4).unwrap()),
                Equal
            );
        }
    }
}

#[test]
fn equipment_comparison_clamps_each_stat_before_summing() {
    let mut data = data();
    let mut actor = data.actors[0].clone();
    actor.curves.attack = vec![995];
    actor.curves.defense = vec![10];
    actor.curves.spirit = vec![10];
    actor.curves.agility = vec![10];
    for (index, attack, defense) in [(0, 10, 0), (1, 500, 0), (2, 10, 1)] {
        let item = &mut data.items[index];
        item.item_type = 1;
        item.atk = attack;
        item.def = defense;
        item.spi = 0;
        item.agi = 0;
    }
    assert_eq!(
        comparison(&actor, 1, &data, [1, 0, 0, 0, 0], data.item(2).unwrap()),
        Equal
    );
    assert_eq!(
        comparison(&actor, 1, &data, [1, 0, 0, 0, 0], data.item(3).unwrap()),
        Greater
    );
}

#[test]
fn unavailable_actor_images_are_cached_grayscale_copies_with_unchanged_alpha() {
    let mut images = Assets::<Image>::default();
    let original = Image {
        data: Some(vec![32, 156, 0, 255, 210, 30, 120, 0]),
        ..default()
    };
    let source = images.add(original.clone());
    let mut cache = Cache::default();
    let handle = gray(&source, &mut images, &mut cache).unwrap();
    assert_eq!(cache.sources.get(&source.id()), Some(&source));
    assert_ne!(handle, source);
    assert_eq!(gray(&source, &mut images, &mut cache), Some(handle.clone()));
    assert_eq!(images.get(&source).unwrap().data, original.data);
    assert_eq!(
        images.get(&handle).unwrap().data.as_ref().unwrap(),
        &[101, 101, 101, 255, 94, 94, 94, 0]
    );
}

#[test]
fn pending_character_textures_remain_owned_until_they_can_be_rasterized() {
    let mut images = Assets::<Image>::default();
    let source = images.reserve_handle();
    let mut cache = Cache::default();
    assert!(gray(&source, &mut images, &mut cache).is_none());
    assert_eq!(cache.sources.get(&source.id()), Some(&source));
    assert!(cache.gray.is_empty());
}

#[test]
fn unloaded_character_images_do_not_draw_a_placeholder_or_a_stale_actor() {
    let mut data = data();
    data.actors[0].character_name = "missing-shop-character-fixture".into();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .insert_resource(data)
        .init_resource::<Party>()
        .init_resource::<Appearance>()
        .init_resource::<Equipment>()
        .init_resource::<Progression>()
        .init_resource::<Cache>()
        .insert_resource(Screen::Shop(Box::new(ShopState {
            items: vec![1],
            allow_buy: true,
            allow_sell: true,
            shop_type: 0,
            phase: Phase::Buy { cursor: 0 },
            scene: default(),
        })))
        .add_systems(Update, update);
    let character = app
        .world_mut()
        .spawn((
            Part::Character(0),
            Node::default(),
            Visibility::Inherited,
            ImageNode::default(),
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(character),
        Some(&Visibility::Hidden)
    );
    assert_eq!(app.world().resource::<Cache>().sources.len(), 1);
}
