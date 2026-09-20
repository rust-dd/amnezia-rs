use super::*;
use gameplay::{app, step};

#[test]
fn standalone_choices_wait_for_the_message_typewriter_before_accepting_input() {
    let mut app = app();
    let mut commands = choices();
    commands[1].string = "\\S[5]abcdefgh".into();
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, commands);
    step(&mut app, &[]);
    assert!(app.world().resource::<Dialogue>().active);
    assert!(!app.world().resource::<Choice>().active());
    for _ in 0..10 {
        step(&mut app, &[KeyCode::Enter, KeyCode::ArrowDown]);
        assert!(app.world().resource::<Dialogue>().active);
        assert!(!app.world().resource::<Choice>().active());
        assert!(!switch_on(&app, 40) && !switch_on(&app, 41));
    }
    for _ in 0..100 {
        step(&mut app, &[]);
        if app.world().resource::<Choice>().active()
            && app.world().resource::<Dialogue>().prompt_input_ready()
        {
            break;
        }
    }
    assert!(app.world().resource::<Choice>().active());
    assert_eq!(app.world().resource::<Choice>().cursor, 0);
    step(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]);
    for _ in 0..3 {
        step(&mut app, &[]);
    }
    assert!(!switch_on(&app, 40));
    assert!(switch_on(&app, 41) && switch_on(&app, 42));
    assert!(!app.world().resource::<Dialogue>().active);
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn standalone_choices_keep_the_remembered_face_and_all_four_text_rows() {
    let mut app = interp_app();
    let mut commands = vec![
        EventCommand {
            string: "Ron".into(),
            ..cmd(10130, 0, vec![6, 0, 0])
        },
        cmd(10140, 0, vec![0]),
    ];
    for index in 0..4 {
        commands.push(EventCommand {
            string: format!("Választás {index}"),
            ..cmd(20140, 0, vec![index])
        });
    }
    commands.push(cmd(20141, 0, vec![]));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, commands);
    app.update();
    let dialogue = app.world().resource::<Dialogue>();
    assert!(dialogue.active);
    assert_eq!(dialogue.boxes.len(), 1);
    assert_eq!(dialogue.boxes[0].face.as_deref(), Some("Ron"));
    assert_eq!(dialogue.boxes[0].face_index, 6);
    assert_eq!(dialogue.boxes[0].lines.len(), 4);
    assert_eq!(dialogue.boxes[0].lines[0], "Választás 0");
    assert!(!app.world().resource::<Choice>().active());
}

#[test]
fn standalone_choice_control_codes_wait_expand_and_cannot_auto_select() {
    let mut app = app();
    app.world_mut().resource_mut::<Variables>().set(8, 12);
    let mut commands = choices();
    commands[1].string = "\\N[1]\\! \\V[8]\\^".into();
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, commands);
    for _ in 0..30 {
        step(&mut app, &[]);
    }
    assert!(app.world().resource::<Dialogue>().active);
    assert!(!app.world().resource::<Choice>().active());
    step(&mut app, &[KeyCode::Enter]);
    for _ in 0..30 {
        step(&mut app, &[]);
    }
    let world = app.world();
    assert!(world.resource::<Dialogue>().active);
    assert!(world.resource::<Dialogue>().prompt_input_ready());
    assert!(world.resource::<Choice>().active());
    assert_eq!(world.resource::<Choice>().options, ["Ron 12", "Nem"]);
    assert!(!switch_on(&app, 40) && !switch_on(&app, 41));
    step(&mut app, &[KeyCode::Escape]);
    for _ in 0..3 {
        step(&mut app, &[]);
    }
    assert!(!switch_on(&app, 40));
    assert!(switch_on(&app, 41) && switch_on(&app, 42));
    assert!(!app.world().resource::<Dialogue>().active);
}

#[test]
fn an_empty_standalone_choice_is_skipped_without_leaving_a_message_owner() {
    let mut app = interp_app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            cmd(10140, 0, vec![0]),
            cmd(20141, 0, vec![]),
            switch_cmd(42, 0, 0),
        ],
    );
    app.update();
    assert!(switch_on(&app, 42));
    assert!(!app.world().resource::<Dialogue>().active);
    assert!(!app.world().resource::<Choice>().active());
    assert!(!app.world().resource::<RunningEvent>().active());
}
