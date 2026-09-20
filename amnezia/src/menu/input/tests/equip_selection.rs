use super::*;

fn equipped(fixed: bool) -> App {
    let mut app = app_on(
        2,
        MenuScreen::Equip {
            member: 0,
            slot: 0,
            picking: None,
        },
    );
    let mut data = app.world_mut().resource_mut::<GameData>();
    data.actors[0].fix_equipment = fixed;
    data.actors[0].weapon = 10;
    data.items = vec![
        testkit::weapon(10, "Rövidkard", 4),
        testkit::weapon(12, "Hosszúkard", 12),
    ];
    app.world_mut().resource_mut::<Inventory>().add_item(12, 1);
    let sound = |name: &str| SoundDef {
        name: name.into(),
        volume: 100,
        tempo: 100,
        ..default()
    };
    app.insert_resource(SystemSounds {
        cursor: sound("CURSOR"),
        decision: sound("DECISION"),
        cancel: sound("CANCEL"),
        buzzer: sound("BUZZER"),
        item: sound("ITEM"),
    });
    app
}

fn sounds(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

#[test]
fn fixed_equipment_refuses_the_picker_with_only_the_buzzer() {
    let mut app = equipped(true);
    let before = app.world().resource::<MenuState>().screen;
    press_frame(&mut app, KeyCode::Enter);
    assert_eq!(app.world().resource::<MenuState>().screen, before);
    assert_eq!(
        sounds(&mut app),
        [AudioRequest::se("BUZZER", 100, 100).unwrap()]
    );
}

#[test]
fn the_first_candidate_is_held_equipment_instead_of_accidental_unequip() {
    let mut app = equipped(false);
    press_frame(&mut app, KeyCode::Enter);
    assert_eq!(
        sounds(&mut app),
        [AudioRequest::se("DECISION", 100, 100).unwrap()]
    );
    press_frame(&mut app, KeyCode::Enter);
    let data = app.world().resource::<GameData>();
    assert_eq!(
        app.world().resource::<Equipment>().slots(&data.actors[0])[0],
        12
    );
    assert_eq!(app.world().resource::<Inventory>().count(10), 1);
    assert_eq!(app.world().resource::<Inventory>().count(12), 0);
    assert_eq!(
        sounds(&mut app),
        [AudioRequest::se("DECISION", 100, 100).unwrap()]
    );
}

#[test]
fn choosing_the_empty_last_entry_returns_the_worn_item_once() {
    let mut app = equipped(false);
    app.world_mut().resource_mut::<MenuState>().screen = MenuScreen::Equip {
        member: 0,
        slot: 0,
        picking: Some(1),
    };
    press_frame(&mut app, KeyCode::Space);
    let data = app.world().resource::<GameData>();
    assert_eq!(
        app.world().resource::<Equipment>().slots(&data.actors[0])[0],
        0
    );
    assert_eq!(app.world().resource::<Inventory>().count(10), 1);
    assert_eq!(app.world().resource::<Inventory>().count(12), 1);
    assert_eq!(
        sounds(&mut app),
        [AudioRequest::se("DECISION", 100, 100).unwrap()]
    );
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::Equip {
            member: 0,
            slot: 0,
            picking: None
        }
    );
}

#[test]
fn selecting_empty_for_an_empty_slot_still_plays_one_decision() {
    let mut app = equipped(false);
    app.world_mut().resource_mut::<MenuState>().screen = MenuScreen::Equip {
        member: 0,
        slot: 1,
        picking: Some(0),
    };
    press_frame(&mut app, KeyCode::Enter);
    assert_eq!(
        sounds(&mut app),
        [AudioRequest::se("DECISION", 100, 100).unwrap()]
    );
    assert_eq!(app.world().resource::<Inventory>().count(12), 1);
    assert!(app.world().resource::<Equipment>().entries().is_empty());
}

#[test]
fn cancelling_the_picker_wins_over_confirm_without_changing_inventory() {
    let mut app = equipped(false);
    press_frame(&mut app, KeyCode::Enter);
    sounds(&mut app);
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    keys.press(KeyCode::Enter);
    keys.press(KeyCode::Escape);
    app.update();
    assert_eq!(
        sounds(&mut app),
        [AudioRequest::se("CANCEL", 100, 100).unwrap()]
    );
    assert_eq!(app.world().resource::<Inventory>().count(12), 1);
    assert!(app.world().resource::<Equipment>().entries().is_empty());
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::Equip {
            member: 0,
            slot: 0,
            picking: None
        }
    );
}
