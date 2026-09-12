use super::*;

fn app(raw: &str, finished: bool) -> App {
    let mut app = App::new();
    InputPrompts::register(&mut app);
    app.insert_resource(MapData::for_test(20, 15))
        .insert_resource(MapEvents { events: Vec::new() })
        .init_resource::<Switches>()
        .init_resource::<Variables>()
        .init_resource::<Party>()
        .init_resource::<Inventory>()
        .init_resource::<Dialogue>()
        .init_resource::<RunningEvent>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, interact);
    let mut reveal = Typewriter::new(raw, "Ron", &Variables::default());
    reveal.tick();
    if finished {
        for _ in 0..200 {
            if reveal.is_complete() {
                break;
            }
            reveal.tick();
        }
        assert!(reveal.is_complete());
    }
    let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
    dialogue.open(vec![MessageBox {
        face: None,
        face_index: 0,
        lines: vec![raw.into()],
    }]);
    dialogue.reveal = Some(reveal);
    app
}

fn press(app: &mut App, key: KeyCode) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    *keys = ButtonInput::default();
    keys.press(key);
    app.update();
}

#[test]
fn decision_during_reveal_does_not_skip_the_original_typing_or_pause() {
    for key in [KeyCode::Enter, KeyCode::Space] {
        for raw in ["abcdefghijklmnop", "ab\\|c"] {
            let mut app = app(raw, false);
            let before = app
                .world()
                .resource::<Dialogue>()
                .reveal
                .as_ref()
                .unwrap()
                .text()
                .to_string();
            press(&mut app, key);
            let dialogue = app.world().resource::<Dialogue>();
            assert!(dialogue.active);
            let reveal = dialogue.reveal.as_ref().unwrap();
            assert_eq!(reveal.text(), before);
            assert!(!reveal.is_complete());
        }
    }
}

#[test]
fn cancel_advances_a_completed_page_like_decision() {
    for key in [KeyCode::Enter, KeyCode::Space, KeyCode::Escape] {
        let mut app = app("Ron", true);
        press(&mut app, key);
        assert!(!app.world().resource::<Dialogue>().active, "{key:?}");
    }
}

#[test]
fn paused_scenes_do_not_dismiss_the_underlying_message() {
    for key in [KeyCode::Enter, KeyCode::Space] {
        let mut app = app("Ron", true);
        app.insert_resource(crate::menu::MenuOpen(true));
        press(&mut app, key);
        assert!(app.world().resource::<Dialogue>().active);
    }
}

#[test]
fn both_keys_release_a_mid_message_key_wait_without_closing_the_page() {
    for key in [KeyCode::Enter, KeyCode::Space, KeyCode::Escape] {
        let mut app = app("a\\!bc", false);
        assert!(
            app.world()
                .resource::<Dialogue>()
                .reveal
                .as_ref()
                .unwrap()
                .waiting_for_key()
        );
        press(&mut app, key);
        let dialogue = app.world().resource::<Dialogue>();
        assert!(dialogue.active);
        assert!(!dialogue.reveal.as_ref().unwrap().waiting_for_key());
        assert_eq!(dialogue.reveal.as_ref().unwrap().text(), "a");
    }
}

#[test]
fn page_input_keeps_physical_keys_available_to_event_key_queries() {
    let mut app = app("Ron", true);
    press(&mut app, KeyCode::Escape);
    assert!(!app.world().resource::<Dialogue>().active);
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(KeyCode::Escape)
    );
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::Escape)
    );
    assert!(app.world().resource::<PromptFrame>().active());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    assert!(!app.world().resource::<PromptFrame>().active());
}

fn action_event(app: &mut App) {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0001.ron",
        crate::assets::asset_root()
    ));
    let mut page = map.events[0].pages[0].clone();
    page.layer = 1;
    page.trigger = 0;
    page.condition = default();
    page.commands = vec![amnezia_data::EventCommand {
        code: 11410,
        indent: 0,
        string: String::new(),
        params: vec![100],
    }];
    app.world_mut()
        .resource_mut::<MapEvents>()
        .events
        .push(amnezia_data::Event {
            id: 7,
            name: String::new(),
            x: 10,
            y: 11,
            pages: vec![page],
        });
    app.world_mut().spawn(Player {
        tile_x: 10,
        tile_y: 10,
        dir: crate::tiles::DIR_DOWN,
        frame: 1,
        charset: String::new(),
        index: 0,
    });
}

fn close_nested(
    mut choice: ResMut<crate::choice::Choice>,
    mut number: ResMut<crate::inputnumber::InputNumber>,
) {
    choice.active = false;
    number.active = false;
}

#[test]
fn closing_a_nested_prompt_does_not_start_a_facing_event_on_the_same_decision() {
    for number in [false, true] {
        let mut app = app("Ron", true);
        app.world_mut().resource_mut::<Dialogue>().close();
        action_event(&mut app);
        app.init_resource::<crate::choice::Choice>()
            .init_resource::<crate::inputnumber::InputNumber>()
            .add_systems(Update, close_nested.before(interact));
        if number {
            app.world_mut()
                .resource_mut::<crate::inputnumber::InputNumber>()
                .open(2, 1);
        } else {
            app.world_mut()
                .resource_mut::<crate::choice::Choice>()
                .open(vec!["Igen".into()], 0, 1);
        }
        press(&mut app, KeyCode::Enter);
        assert!(!app.world().resource::<RunningEvent>().active());
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(7));
    }
}
