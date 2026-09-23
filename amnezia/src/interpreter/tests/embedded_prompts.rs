use super::*;

mod gameplay;
mod standalone;

fn message(text: &str, continuation: bool) -> EventCommand {
    EventCommand {
        string: text.into(),
        ..cmd(if continuation { 20110 } else { 10110 }, 0, vec![])
    }
}

fn choices() -> Vec<EventCommand> {
    vec![
        cmd(10140, 0, vec![2]),
        EventCommand {
            string: "Igen".into(),
            ..cmd(20140, 0, vec![0])
        },
        switch_cmd(40, 0, 1),
        EventCommand {
            string: "Nem".into(),
            ..cmd(20140, 0, vec![1])
        },
        switch_cmd(41, 0, 1),
        cmd(20141, 0, vec![]),
        switch_cmd(42, 0, 0),
    ]
}

#[test]
fn fitting_choices_are_appended_to_the_message_and_reserved_by_its_owner() {
    let mut app = interp_app();
    let mut commands = vec![message("Kérdés", false), message("Második sor", true)];
    commands.extend(choices());
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, commands);
    app.update();
    let dialogue = app.world().resource::<Dialogue>();
    assert_eq!(
        dialogue.boxes[0].lines,
        ["Kérdés", "Második sor", "Igen", "Nem"]
    );
    let frame = &app.world().resource::<RunningEvent>().frame;
    assert_eq!(frame.ip, 2);
    assert!(frame.message_pending && frame.choice_pending);
    assert!(!app.world().resource::<Choice>().active());
}

#[test]
fn a_number_row_is_reserved_after_up_to_three_message_lines() {
    for lines in 1..=3 {
        let mut app = interp_app();
        let mut commands = (0..lines)
            .map(|row| message("Írj be egy számot!", row > 0))
            .collect::<Vec<_>>();
        commands.push(cmd(10150, 0, vec![4, 76]));
        commands.push(switch_cmd(42, 0, 0));
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(1, commands);
        app.update();
        let frame = &app.world().resource::<RunningEvent>().frame;
        assert_eq!(frame.ip, lines);
        assert!(
            frame.message_pending && frame.input_pending,
            "{lines} lines"
        );
        assert!(!app.world().resource::<InputNumber>().active());
        assert_eq!(
            app.world().resource::<Dialogue>().boxes[0].lines.len(),
            lines
        );
    }
}

#[test]
fn prompts_do_not_attach_when_the_message_is_full_or_a_face_command_intervenes() {
    for numeric in [false, true] {
        for face_barrier in [false, true] {
            let mut app = interp_app();
            let lines = if face_barrier { 1 } else { 4 };
            let mut commands = (0..lines)
                .map(|row| message("Szöveg", row > 0))
                .collect::<Vec<_>>();
            if face_barrier {
                commands.push(cmd(10130, 0, vec![0]));
            }
            commands.extend(if numeric {
                vec![cmd(10150, 0, vec![1, 24])]
            } else {
                choices()
            });
            app.world_mut()
                .resource_mut::<RunningEvent>()
                .start(1, commands);
            app.update();
            let frame = &app.world().resource::<RunningEvent>().frame;
            assert!(frame.message_pending);
            assert!(!frame.choice_pending && !frame.input_pending);
            assert_eq!(
                app.world().resource::<Dialogue>().boxes[0].lines.len(),
                lines
            );
        }
    }
}

#[test]
fn choices_only_attach_to_the_last_message_page_and_must_fit_completely() {
    for lines in 1..=3 {
        let mut app = interp_app();
        let mut commands = vec![message("Első oldal", false)];
        commands.extend((0..lines).map(|row| message("Utolsó oldal", row > 0)));
        commands.extend(choices());
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(1, commands);
        app.update();
        assert_eq!(
            app.world().resource::<Dialogue>().boxes[0].lines,
            ["Első oldal"]
        );
        assert_eq!(app.world().resource::<Dialogue>().boxes.len(), 1);
        assert!(!app.world().resource::<RunningEvent>().frame.choice_pending);
        app.world_mut().resource_mut::<Dialogue>().close();
        app.update();
        let dialogue = app.world().resource::<Dialogue>();
        assert_eq!(
            dialogue.boxes[0].lines.len(),
            lines + if lines <= 2 { 2 } else { 0 }
        );
        assert_eq!(
            app.world().resource::<RunningEvent>().frame.choice_pending,
            lines <= 2
        );
    }
}

#[test]
fn all_original_numeric_prompts_preserve_their_adjacent_message_rows() {
    let mut total = 0;
    let mut embedded = 0;
    for id in [102, 106, 182, 184, 189, 191, 194, 196, 198, 247, 256] {
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/maps/map_{id:04}.ron",
            crate::assets::asset_root()
        ));
        for page in map.events.iter().flat_map(|event| &event.pages) {
            for (index, command) in page
                .commands
                .iter()
                .enumerate()
                .filter(|(_, command)| command.code == 10150)
            {
                total += 1;
                let mut first = index;
                while first > 0 && page.commands[first - 1].code == 20110 {
                    first -= 1;
                }
                if first > 0 && page.commands[first - 1].code == 10110 {
                    first -= 1;
                } else {
                    first = index;
                }
                let mut app = interp_app();
                app.world_mut()
                    .resource_mut::<RunningEvent>()
                    .start(1, page.commands[first..=index].to_vec());
                app.update();
                let rows = index - first;
                let dialogue = app.world().resource::<Dialogue>();
                if rows == 0 {
                    assert!(dialogue.active);
                    assert!(dialogue.boxes[0].lines.is_empty());
                    assert!(app.world().resource::<InputNumber>().active());
                } else {
                    assert_eq!(dialogue.boxes[0].lines.len(), rows);
                    let pending = app.world().resource::<RunningEvent>().frame.input_pending;
                    assert_eq!(pending, rows < 4, "map {id}, {:?}", command.params);
                    embedded += usize::from(pending);
                }
            }
        }
    }
    assert_eq!(total, 33);
    assert!(embedded > 0);
    eprintln!(
        "original number prompts: {embedded} embedded, {} standalone",
        total - embedded
    );
}
