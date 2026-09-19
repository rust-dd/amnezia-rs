use super::*;
use crate::dialogue::saved::{Capture, MessageState};
use crate::dialogue::{MessageOptions, MessagePosition, MessageTransparent};
use crate::events::message_boxes;
use bevy::ecs::system::RunSystemOnce;

fn command(code: u32, text: &str, params: Vec<i32>) -> amnezia_data::EventCommand {
    amnezia_data::EventCommand {
        code,
        indent: 0,
        string: text.into(),
        params,
    }
}

fn message_app(path: PathBuf) -> App {
    let mut app = save_app(path);
    app.init_resource::<RunningEvent>()
        .init_resource::<MessagePosition>()
        .init_resource::<MessageTransparent>()
        .init_resource::<MessageOptions>();
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
    app
}

#[test]
fn saved_message_placement_transparency_and_event_options_survive_reload_cleanup() {
    let path = temp_slot("message_options");
    let mut app = message_app(path.clone());
    app.insert_resource(MessagePosition::Top);
    app.world_mut().resource_mut::<MessageTransparent>().0 = true;
    app.insert_resource(MessageOptions {
        fixed: true,
        continue_events: true,
    });
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    app.insert_resource(MessagePosition::Middle);
    app.insert_resource(MessageTransparent::default());
    app.insert_resource(MessageOptions::default());
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    std::fs::remove_file(path).unwrap();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    assert_eq!(
        *app.world().resource::<MessagePosition>(),
        MessagePosition::Top
    );
    assert!(app.world().resource::<MessageTransparent>().0);
    assert_eq!(
        *app.world().resource::<MessageOptions>(),
        MessageOptions {
            fixed: true,
            continue_events: true,
        }
    );
}

#[test]
fn a_saved_portrait_applies_to_the_next_message_without_restoring_a_stale_open_box() {
    let path = temp_slot("message_face");
    let mut app = message_app(path.clone());
    message_boxes(
        &[command(10130, "Ron", vec![6, 0, 0])],
        &mut app.world_mut().resource_mut::<Dialogue>().face,
    );
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    {
        let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
        let boxes = message_boxes(
            &[
                command(10130, "", vec![]),
                command(10110, "Old session", vec![]),
            ],
            &mut dialogue.face,
        );
        dialogue.open(boxes);
    }
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    std::fs::remove_file(path).unwrap();
    let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
    assert!(!dialogue.active);
    assert!(dialogue.boxes.is_empty());
    let boxes = message_boxes(&[command(10110, "New session", vec![])], &mut dialogue.face);
    assert_eq!(boxes[0].face.as_deref(), Some("Ron"));
    assert_eq!(boxes[0].face_index, 6);
}

fn snapshot(world: &mut World) -> MessageState {
    world
        .run_system_once(|capture: Capture, dialogue: Res<Dialogue>| capture.snapshot(&dialogue))
        .unwrap()
}

#[test]
fn all_saved_message_options_and_an_explicitly_cleared_face_round_trip_through_the_file() {
    for position in [
        MessagePosition::Top,
        MessagePosition::Middle,
        MessagePosition::Bottom,
    ] {
        for bits in 0..16 {
            let path = temp_slot(&format!("message_matrix_{position:?}_{bits}"));
            let mut app = message_app(path.clone());
            app.insert_resource(position);
            app.insert_resource(MessageTransparent(bits & 1 != 0));
            app.insert_resource(MessageOptions {
                fixed: bits & 2 != 0,
                continue_events: bits & 4 != 0,
            });
            message_boxes(
                &[command(
                    10130,
                    if bits & 8 == 0 { "" } else { "Ron" },
                    vec![6],
                )],
                &mut app.world_mut().resource_mut::<Dialogue>().face,
            );
            let expected = snapshot(app.world_mut());
            app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
            app.update();
            assert_eq!(read_save(&path).unwrap().message, expected);
            let bytes = std::fs::read(&path).unwrap();
            message_boxes(
                &[command(10130, "Tiffany", vec![5])],
                &mut app.world_mut().resource_mut::<Dialogue>().face,
            );
            app.world_mut().resource_mut::<LoadRequest>().0 = true;
            app.update();
            assert_eq!(snapshot(app.world_mut()), expected);
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            std::fs::remove_file(path).unwrap();
        }
    }
}

#[test]
fn legacy_saves_reset_message_settings_to_original_defaults_without_rewriting_the_file() {
    for version in 0..3 {
        let path = temp_slot(&format!("message_legacy_{version}"));
        let original = format!(
            "(format_version:{version},map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:0)"
        );
        std::fs::write(&path, &original).unwrap();
        let mut app = message_app(path.clone());
        app.insert_resource(MessagePosition::Top);
        app.insert_resource(MessageTransparent(true));
        app.insert_resource(MessageOptions {
            fixed: true,
            continue_events: true,
        });
        message_boxes(
            &[command(10130, "Ron", vec![6])],
            &mut app.world_mut().resource_mut::<Dialogue>().face,
        );
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert_eq!(snapshot(app.world_mut()), MessageState::default());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn invalid_saved_message_placement_leaves_the_live_session_and_file_unchanged() {
    let path = temp_slot("message_invalid");
    let mut app = message_app(path.clone());
    app.insert_resource(MessagePosition::Top);
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    let original = std::fs::read_to_string(&path).unwrap();
    let invalid = original.replace("position: Top", "position: Nowhere");
    assert_ne!(invalid, original);
    std::fs::write(&path, &invalid).unwrap();
    app.insert_resource(MessagePosition::Middle);
    app.world_mut().resource_mut::<Switches>().set(99, true);
    let expected = snapshot(app.world_mut());
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(false));
    assert_eq!(snapshot(app.world_mut()), expected);
    assert!(app.world().resource::<Switches>().get(99));
    assert!(app.world().resource::<PendingTeleport>().0.is_none());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn starting_a_new_game_clears_previously_loaded_message_settings() {
    let path = temp_slot("message_new_game");
    let mut app = message_app(path.clone());
    app.add_plugins(crate::session::SessionPlugin);
    app.insert_resource(MessagePosition::Top);
    app.insert_resource(MessageTransparent(true));
    message_boxes(
        &[command(10130, "Ron", vec![6])],
        &mut app.world_mut().resource_mut::<Dialogue>().face,
    );
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(snapshot(app.world_mut()), read_save(&path).unwrap().message);
    app.world_mut()
        .resource_mut::<crate::session::NewGameRequest>()
        .requested = true;
    app.update();
    assert_eq!(snapshot(app.world_mut()), MessageState::default());
    std::fs::remove_file(path).unwrap();
}
