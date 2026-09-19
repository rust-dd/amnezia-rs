use super::*;
use crate::menu::testkit;
use crate::save::SaveRequest;

mod guards;
mod item_list;
mod message_frames;

/// A headless app with just the menu input system and the resources it reads,
/// opened on `screen` with `cursor` as the command-list cursor.
fn app_on(cursor: usize, screen: MenuScreen) -> App {
    let mut app = App::new();
    app.insert_resource(testkit::data())
        .init_resource::<Party>()
        .init_resource::<Progression>()
        .init_resource::<Inventory>()
        .init_resource::<Vitals>()
        .init_resource::<Equipment>()
        .insert_resource(ShopOpen(false))
        .insert_resource(BattleActive(false))
        .insert_resource(TitleActive(false))
        .init_resource::<MenuAccess>()
        .init_resource::<SaveAccess>()
        .insert_resource(MenuOpen(true))
        .insert_resource(MenuState { cursor, screen })
        .init_resource::<SaveRequest>()
        .init_resource::<SaveFiles>()
        .init_resource::<items::List>()
        .init_resource::<crate::timing::GameFrames>()
        .init_resource::<Dialogue>()
        .init_resource::<RunningEvent>()
        .init_resource::<Choice>()
        .init_resource::<InputNumber>()
        .init_resource::<Fade>()
        .init_resource::<GameOverActive>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<AudioRequest>()
        .add_systems(Update, (items::update, menu_input).chain());
    app
}

/// Press a key and run one input frame.
fn confirm(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
}

/// Release every held key, press `key` afresh, and run one frame — so a
/// multi-step interaction sees a genuine just-pressed each step (no input plugin
/// runs to reset it in these headless apps, and `press` only re-arms
/// `just_pressed` for a newly held key).
fn press_frame(app: &mut App, key: KeyCode) {
    {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.reset_all();
        input.press(key);
    }
    app.update();
}

#[test]
fn save_command_waits_for_a_slot_before_requesting_a_save() {
    let mut app = app_on(3, MenuScreen::Command);
    app.world_mut().insert_resource(SaveAccess(true));
    confirm(&mut app, KeyCode::Enter);
    assert!(
        !app.world().resource::<SaveRequest>().0,
        "entering the file selector must not write the first slot"
    );
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::Command,
        "the command cursor remains underneath the modal file selector"
    );
}

#[test]
fn save_shortcut_waits_for_a_slot_before_requesting_a_save() {
    let mut app = app_on(3, MenuScreen::Command);
    app.world_mut().insert_resource(SaveAccess(true));
    confirm(&mut app, KeyCode::KeyS);
    assert!(!app.world().resource::<SaveRequest>().0);
}

#[test]
fn menu_stays_closed_when_menu_access_is_disabled() {
    let mut app = app_on(0, MenuScreen::Command);
    app.world_mut().insert_resource(MenuOpen(false));
    app.world_mut().insert_resource(MenuAccess(false));
    confirm(&mut app, KeyCode::Escape);
    assert!(
        !app.world().resource::<MenuOpen>().0,
        "the menu must stay closed while access is disabled"
    );
}

#[test]
fn menu_will_not_open_while_a_dialogue_is_active() {
    let mut app = app_on(0, MenuScreen::Command);
    app.world_mut().insert_resource(MenuOpen(false));
    app.world_mut().resource_mut::<Dialogue>().active = true;
    confirm(&mut app, KeyCode::Escape);
    assert!(
        !app.world().resource::<MenuOpen>().0,
        "the menu must refuse to open while a dialogue is showing"
    );
}

#[test]
fn open_menu_still_closes_while_a_blocker_would_forbid_opening() {
    // The guard gates opening only: an already-open menu closes on Escape even
    // if a transient flow (here a running event) is flagged active.
    let mut app = app_on(0, MenuScreen::Command);
    app.world_mut().resource_mut::<Dialogue>().active = true;
    confirm(&mut app, KeyCode::Escape);
    assert!(
        !app.world().resource::<MenuOpen>().0,
        "an open menu must still close despite an active blocker"
    );
}

#[test]
fn save_command_is_inert_when_save_access_is_disabled() {
    let mut app = app_on(3, MenuScreen::Command);
    app.world_mut().insert_resource(SaveAccess(false));
    confirm(&mut app, KeyCode::Enter);
    assert!(
        !app.world().resource::<SaveRequest>().0,
        "Save must be inert while save access is disabled"
    );
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::Command,
        "and the menu stays on the command list"
    );
}

#[test]
fn the_equip_screen_swaps_gear_through_the_runtime_store() {
    let mut app = app_on(
        0,
        MenuScreen::Equip {
            member: 0,
            slot: 0,
            picking: None,
        },
    );
    {
        let mut data = app.world_mut().resource_mut::<GameData>();
        data.actors[0].weapon = 10;
        data.items.push(testkit::weapon(10, "Rövidkard", 4));
        data.items.push(testkit::weapon(12, "Hosszúkard", 12));
    }
    app.world_mut().resource_mut::<Inventory>().add_item(12, 1);

    press_frame(&mut app, KeyCode::Enter);
    assert!(
        matches!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::Equip {
                picking: Some(_),
                ..
            }
        ),
        "confirming a slot opens the item picker"
    );
    press_frame(&mut app, KeyCode::ArrowDown);
    press_frame(&mut app, KeyCode::Enter);

    assert!(matches!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::Equip { picking: None, .. }
    ));
    let worn = {
        let data = app.world().resource::<GameData>();
        let def = data.actor(1).unwrap();
        app.world().resource::<Equipment>().slots(def)[0]
    };
    assert_eq!(worn, 12, "the long-sword is now worn");
    let inv = app.world().resource::<Inventory>();
    assert_eq!(inv.count(12), 0, "it left the bag");
    assert_eq!(inv.count(10), 1, "the short-sword returned to the bag");
}

#[test]
fn end_game_igen_closes_the_menu_and_raises_the_title() {
    let mut app = app_on(4, MenuScreen::EndGame { cursor: 0 });
    confirm(&mut app, KeyCode::Enter);
    assert!(!app.world().resource::<MenuOpen>().0, "menu closed");
    assert!(
        app.world().resource::<TitleActive>().0,
        "title raised (return-to-title, opcode 12510)"
    );
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::Command,
        "and the menu resets to the command list for next time"
    );
}
