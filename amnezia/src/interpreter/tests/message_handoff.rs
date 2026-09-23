use super::*;
use crate::dialogue::testing::{dismiss, finish_window_close};

fn message(text: &str) -> EventCommand {
    EventCommand {
        string: text.into(),
        ..cmd(10110, 0, vec![])
    }
}

#[test]
fn adjacent_message_commands_do_not_run_the_next_face_or_options_early() {
    let mut app = interp_app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            message("Első"),
            switch_cmd(42, 0, 0),
            EventCommand {
                string: "Ron".into(),
                ..cmd(10130, 0, vec![6, 0, 0])
            },
            cmd(10120, 0, vec![0, 0, 1, 0]),
            message("Második"),
        ],
    );
    app.update();
    assert_eq!(app.world().resource::<Dialogue>().boxes.len(), 1);
    assert!(!switch_on(&app, 42));
    assert!(app.world().resource::<Dialogue>().face.graphic().is_none());
    dismiss(app.world_mut());
    app.update();
    assert!(switch_on(&app, 42));
    let dialogue = app.world().resource::<Dialogue>();
    assert_eq!(dialogue.boxes[0].lines, ["Második"]);
    assert_eq!(dialogue.boxes[0].face.as_deref(), Some("Ron"));
    assert_eq!(dialogue.boxes[0].face_index, 6);
    assert!(dialogue.lifecycle.message.ready());
    assert_eq!(
        *app.world().resource::<MessagePosition>(),
        MessagePosition::Top
    );
}

#[test]
fn a_parallel_followup_waits_for_closing_and_then_opens_from_zero_height() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![map_event(
            1,
            4,
            vec![message("Első"), switch_cmd(42, 0, 0), message("Második")],
        )],
    });
    app.update();
    dismiss(app.world_mut());
    app.update();
    assert!(!switch_on(&app, 42));
    assert!(!app.world().resource::<Dialogue>().active);
    finish_window_close(app.world_mut());
    app.update();
    assert!(switch_on(&app, 42));
    let dialogue = app.world().resource::<Dialogue>();
    assert_eq!(dialogue.boxes[0].lines, ["Második"]);
    assert_eq!(dialogue.lifecycle.message.half_height(80), 0);
}

#[test]
fn a_paid_foreground_inn_only_uses_the_immediate_message_handoff() {
    for delayed in [false, true] {
        let mut app = interp_app();
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(1, vec![message("Első"), cmd(10730, 0, vec![1, 30, 1])]);
        app.update();
        dismiss(app.world_mut());
        if delayed {
            app.world_mut()
                .resource_mut::<crate::timing::GameFrames>()
                .frame += 1;
            crate::dialogue::testing::tick(app.world_mut());
        }
        app.update();
        assert_eq!(
            app.world().resource::<Messages<ShopRequest>>().len(),
            usize::from(!delayed)
        );
        if delayed {
            finish_window_close(app.world_mut());
            app.update();
            assert_eq!(app.world().resource::<Messages<ShopRequest>>().len(), 1);
        }
    }
}
