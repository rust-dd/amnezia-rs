use super::*;
use bevy::prelude::*;

#[test]
fn a_completed_cellar_autorun_does_not_disable_player_input() {
    let map = load_ron::<Map>(&format!("{}/maps/map_0045.ron", asset_root()));
    let event = map.events.into_iter().find(|event| event.id == 9).unwrap();
    let mut app = crate::world::test_support::app(vec![event], false);
    app.world_mut()
        .resource_mut::<crate::state::Switches>()
        .set(111, true);
    for _ in 0..4 {
        app.update();
    }
    let world = app.world_mut();
    let origin = world
        .query::<&crate::player::Player>()
        .single(world)
        .unwrap()
        .tile_x;
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    for _ in 0..4 {
        app.update();
    }
    let world = app.world_mut();
    assert!(
        world
            .query::<&crate::player::Player>()
            .single(world)
            .unwrap()
            .tile_x
            > origin,
        "an original empty autorun must not steal every player update"
    );
}

#[test]
fn original_empty_map_and_common_pages_deserialize_as_empty_command_lists() {
    let map = load_ron::<Map>(&format!("{}/maps/map_0045.ron", asset_root()));
    assert!(
        map.events.iter().find(|event| event.id == 9).unwrap().pages[1]
            .commands
            .is_empty()
    );
    let common = load_ron::<Vec<CommonEvent>>(&format!("{}/common_events.ron", asset_root()));
    assert!(
        common
            .iter()
            .all(|event| event.commands.iter().all(|command| command.code != 0))
    );
}

#[test]
fn legacy_ron_end_markers_are_removed_without_dropping_real_branch_ends() {
    let mut event = load_ron::<Map>(&format!("{}/maps/map_0003.ron", asset_root()))
        .events
        .remove(0);
    let commands = [10, 22011, 0, 10210]
        .map(|code| EventCommand {
            code,
            indent: 0,
            string: String::new(),
            params: vec![],
        })
        .to_vec();
    event.pages[0].commands = commands.clone();
    let decoded = ron::from_str::<amnezia_data::Event>(&ron::to_string(&event).unwrap()).unwrap();
    assert_eq!(
        decoded.pages[0]
            .commands
            .iter()
            .map(|c| c.code)
            .collect::<Vec<_>>(),
        [10, 22011]
    );
    let common = CommonEvent {
        id: 1,
        name: String::new(),
        trigger: 4,
        switch_flag: false,
        switch_id: 1,
        commands: commands.clone(),
    };
    let decoded = ron::from_str::<CommonEvent>(&ron::to_string(&common).unwrap()).unwrap();
    assert_eq!(
        decoded.commands.iter().map(|c| c.code).collect::<Vec<_>>(),
        [10, 22011]
    );
    let troop = amnezia_data::TroopPageDef {
        commands,
        ..default()
    };
    let decoded =
        ron::from_str::<amnezia_data::TroopPageDef>(&ron::to_string(&troop).unwrap()).unwrap();
    assert_eq!(
        decoded.commands.iter().map(|c| c.code).collect::<Vec<_>>(),
        [10, 22011]
    );
}
