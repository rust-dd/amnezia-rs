use super::*;
use crate::menu::{MenuAccess, MenuOpen, MenuScreen, MenuState};
use crate::save::{SaveAccess, SaveRequest, slots::ActiveSlot};

fn app(tag: &str) -> (App, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!("amnezia_{tag}_{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let mut app = crate::save::tests::save_resources(directory.join("slot1.ron"));
    app.add_plugins((
        AssetPlugin::default(),
        crate::save::SavePlugin,
        crate::gamedata::GameDataPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<crate::interpreter::RunningEvent>()
    .init_resource::<crate::choice::Choice>()
    .init_resource::<crate::inputnumber::InputNumber>()
    .init_resource::<crate::gameover::GameOverActive>()
    .init_resource::<crate::battle::BattleActive>()
    .init_resource::<crate::shop::ShopOpen>()
    .insert_resource(crate::title::TitleActive(false))
    .init_resource::<MenuAccess>()
    .insert_resource(MenuOpen(true))
    .insert_resource(MenuState {
        cursor: 3,
        screen: MenuScreen::Command,
    })
    .insert_resource(SaveAccess(true))
    .init_resource::<SaveFiles>()
    .add_message::<crate::audio::AudioRequest>()
    .add_systems(
        Update,
        crate::menu::input::menu_input.in_set(crate::menu::MenuInput),
    );
    super::register_flow(&mut app);
    let mut map = crate::world::MapData::for_test(20, 15);
    map.map_id = 2;
    app.insert_resource(map);
    app.world_mut().spawn(crate::player::Player {
        tile_x: 3,
        tile_y: 4,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    (app, directory)
}

fn step(app: &mut App, key: Option<KeyCode>) {
    let world = app.world_mut();
    world.resource_mut::<crate::timing::GameFrames>().frame += 1;
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if let Some(key) = key {
        keys.press(key);
    }
    app.update();
}

#[test]
fn cancel_returns_to_the_same_command_without_writing_or_closing_the_menu() {
    let (mut app, directory) = app("file_selector_cancel");
    step(&mut app, Some(KeyCode::Enter));
    assert!(app.world().resource::<SaveFiles>().active());
    assert!(!app.world().resource::<SaveRequest>().0);
    step(&mut app, Some(KeyCode::Escape));
    step(&mut app, None);
    assert!(!app.world().resource::<SaveFiles>().active());
    assert!(app.world().resource::<MenuOpen>().0);
    assert_eq!(app.world().resource::<MenuState>().cursor, 3);
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn selecting_the_last_slot_saves_only_that_slot_after_confirmation() {
    let (mut app, directory) = app("file_selector_last");
    step(&mut app, Some(KeyCode::Enter));
    step(&mut app, Some(KeyCode::ArrowUp));
    assert_eq!(app.world().resource::<SaveFiles>().navigation.index, 14);
    step(&mut app, Some(KeyCode::Enter));
    assert!(app.world().resource::<SaveFiles>().active());
    assert!(!app.world().resource::<SaveRequest>().0);
    for _ in 0..8 {
        step(&mut app, None);
    }
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
    step(&mut app, Some(KeyCode::Enter));
    assert!(!app.world().resource::<SaveFiles>().active());
    assert!(app.world().resource::<SaveRequest>().0);
    step(&mut app, None);
    assert_eq!(
        *app.world().resource::<ActiveSlot>(),
        ActiveSlot::new(15).unwrap()
    );
    assert!(directory.join("slot15.ron").is_file());
    assert!(!directory.join("slot1.ron").exists());
    assert!(app.world().resource::<MenuOpen>().0);
    std::fs::remove_file(directory.join("slot15.ron")).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

mod clocks;
mod events;
mod navigation;
