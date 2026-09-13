use super::*;
use crate::gamedata::GameData;

#[test]
fn saved_gear_must_be_equippable_by_its_actor_without_refunding_invalid_slots() {
    let mut definitions = App::new();
    definitions.add_plugins(GameDataPlugin);
    let data = definitions.world().resource::<GameData>();
    let item = data
        .items
        .iter()
        .find(|item| item.item_type == 1 && !item.usable_by_actor(1))
        .unwrap();
    for version in [14, SAVE_FORMAT_VERSION] {
        let (app, path, original) = load_case(
            &format!("restricted_saved_gear_{version}"),
            version,
            |game| {
                game.items = vec![(item.id, 2)];
                game.equipment = vec![(1, [item.id, 0, 0, 0, 0])];
            },
        );
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(version < 15));
        if version < 15 {
            assert_eq!(app.world().resource::<Equipment>().entries(), [(1, [0; 5])]);
            assert_eq!(app.world().resource::<Inventory>().count(item.id), 2);
        } else {
            assert!(app.world().resource::<Switches>().get(888));
            assert!(app.world().resource::<PendingTeleport>().0.is_none());
        }
        unchanged_file(path, original);
    }
}

#[test]
fn invalid_live_rosters_cannot_overwrite_an_existing_slot() {
    let (mut app, path, original) = load_case("invalid_identity_save", SAVE_FORMAT_VERSION, |_| {});
    let mut map = MapData::for_test(20, 15);
    map.map_id = 2;
    app.insert_resource(map);
    app.world_mut().spawn(Player {
        tile_x: 3,
        tile_y: 4,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    app.world_mut().resource_mut::<Party>().restore(vec![1, 1]);
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    assert!(!app.world().resource::<EventSaveRequest>().0);
    assert_eq!(app.world().resource::<Party>().snapshot(), [1, 1]);
    unchanged_file(path, original);
}
