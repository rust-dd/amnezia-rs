use super::*;
use crate::events::{MessageFace, message_boxes};

fn face(name: &str) -> EventCommand {
    EventCommand {
        string: name.into(),
        ..cmd(10130, 0, vec![6, 0, 0])
    }
}

fn message() -> EventCommand {
    EventCommand {
        string: "A message".into(),
        ..cmd(10110, 0, vec![])
    }
}

fn set_face(app: &mut App, name: &str) {
    message_boxes(
        &[face(name)],
        &mut app.world_mut().resource_mut::<Dialogue>().face,
    );
}

fn shown(app: &App) -> Option<&str> {
    app.world().resource::<Dialogue>().boxes[0].face.as_deref()
}

#[test]
fn foreground_start_clears_a_previous_face_before_its_first_message() {
    let mut app = interp_app();
    set_face(&mut app, "Ron");
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, vec![message()]);
    app.update();
    assert_eq!(shown(&app), None);
}

#[test]
fn foreground_completion_clears_only_the_face_not_the_message_options() {
    let mut app = interp_app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        0,
        vec![
            cmd(10120, 0, vec![1, 0, 0, 1]),
            face("Ron"),
            cmd(11410, 0, vec![0]),
        ],
    );
    app.update();
    assert_eq!(
        app.world().resource::<Dialogue>().face.graphic(),
        Some(("Ron", 6))
    );
    app.update();
    assert!(!app.world().resource::<RunningEvent>().active());
    assert_eq!(
        app.world().resource::<Dialogue>().face,
        MessageFace::default()
    );
    assert_eq!(
        *app.world().resource::<MessagePosition>(),
        MessagePosition::Top
    );
    assert!(app.world().resource::<MessageTransparent>().0);
    assert_eq!(
        *app.world().resource::<crate::dialogue::MessageOptions>(),
        crate::dialogue::MessageOptions {
            fixed: true,
            continue_events: true
        }
    );
}

#[test]
fn a_called_event_inherits_and_can_replace_its_callers_face_until_base_completion() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![map_event(2, 0, vec![message(), face("Tiffany")])],
    });
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![face("Ron"), cmd(12330, 0, vec![1, 2, 1]), message()],
    );
    app.update();
    assert_eq!(shown(&app), Some("Ron"));
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert_eq!(shown(&app), Some("Tiffany"));
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert_eq!(
        app.world().resource::<Dialogue>().face,
        MessageFace::default()
    );
}

#[test]
fn parallel_start_completion_and_restart_preserve_the_shared_face() {
    let mut app = interp_app();
    set_face(&mut app, "Ron");
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 4, vec![message()])],
    });
    for _ in 0..2 {
        app.update();
        assert_eq!(shown(&app), Some("Ron"));
        app.world_mut().resource_mut::<Dialogue>().close();
        app.update();
        assert_eq!(
            app.world().resource::<Dialogue>().face.graphic(),
            Some(("Ron", 6))
        );
    }
}

#[test]
fn a_queued_foreground_event_clears_the_face_set_by_an_earlier_parallel_script() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 3, vec![message()]),
            map_event(2, 4, vec![face("Ron"), cmd(11410, 0, vec![100])]),
        ],
    });
    app.update();
    assert_eq!(shown(&app), None);
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(1));
}

#[test]
fn starting_a_foreground_frame_does_not_erase_an_already_open_parallel_portrait() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 3, vec![switch_cmd(10, 0, 0)]),
            map_event(2, 4, vec![face("Ron"), message()]),
        ],
    });
    app.update();
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(1));
    assert!(!switch_on(&app, 10));
    assert_eq!(shown(&app), Some("Ron"));
    assert_eq!(
        app.world().resource::<Dialogue>().face,
        MessageFace::default()
    );
}

#[test]
fn a_map_event_above_a_common_autorun_shares_the_face_until_the_base_frame_finishes() {
    let mut app = interp_app();
    let mut common_commands = vec![switch_cmd(5, 1, 0), message()];
    common_commands.push(switch_cmd(10, 0, 0));
    app.insert_resource(CommonEvents(vec![common(1, 3, 5, common_commands)]));
    app.world_mut().resource_mut::<Switches>().set(5, true);
    let mut event = map_event(1, 3, vec![face("Ron"), switch_cmd(6, 1, 0)]);
    event.pages[0].condition.flags = 1;
    event.pages[0].condition.switch_a = 6;
    app.world_mut().resource_mut::<Switches>().set(6, true);
    app.insert_resource(MapEvents {
        events: vec![event],
    });
    app.update();
    assert_eq!(shown(&app), Some("Ron"));
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert!(switch_on(&app, 10));
    assert_eq!(
        app.world().resource::<Dialogue>().face,
        MessageFace::default()
    );
}

#[test]
fn a_restored_frame_at_its_first_command_does_not_clear_the_saved_face_again() {
    let mut app = interp_app();
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, vec![message()]);
    let saved = app.world().resource::<RunningEvent>().snapshot();
    set_face(&mut app, "Ron");
    crate::interpreter::saved::restore(app.world_mut(), saved);
    app.update();
    assert_eq!(shown(&app), Some("Ron"));
}

#[test]
fn a_foreground_start_during_battle_leaves_its_portrait_untouched() {
    let mut app = interp_app();
    set_face(&mut app, "Ron");
    app.world_mut().resource_mut::<BattleActive>().0 = true;
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, vec![cmd(11410, 0, vec![0])]);
    app.update();
    assert_eq!(
        app.world().resource::<Dialogue>().face.graphic(),
        Some(("Ron", 6))
    );
    app.world_mut().resource_mut::<BattleActive>().0 = false;
    app.update();
    assert_eq!(
        app.world().resource::<Dialogue>().face.graphic(),
        Some(("Ron", 6))
    );
    app.update();
    assert_eq!(
        app.world().resource::<Dialogue>().face,
        MessageFace::default()
    );
}
