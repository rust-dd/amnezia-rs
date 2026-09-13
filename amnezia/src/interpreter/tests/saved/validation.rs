use super::*;

#[test]
fn old_and_empty_foreground_saves_clear_an_unrelated_event_without_rewriting_the_slot() {
    let (mut app, path) = app("legacy");
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, vec![cmd(11910, 0, vec![])]);
    app.update();
    app.update();
    let original = std::fs::read_to_string(&path).unwrap();
    assert!(
        original
            .lines()
            .any(|line| line.starts_with("    foreground:"))
    );
    for version in 0..=crate::save::SAVE_FORMAT_VERSION {
        let content = original
            .lines()
            .take_while(|line| !line.starts_with("    foreground:"))
            .map(|line| {
                if line.starts_with("    format_version:") {
                    format!("    format_version: {version},")
                } else {
                    line.to_owned()
                }
            })
            .chain(std::iter::once(")".into()))
            .collect::<Vec<_>>()
            .join("\n");
        let content = if version == crate::save::SAVE_FORMAT_VERSION {
            content.replace("\n)", "\n    foreground: None,\n)")
        } else {
            content
        };
        std::fs::write(&path, &content).unwrap();
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(99, vec![switch_cmd(9998, 0, 0), cmd(11910, 0, vec![])]);
        load(&mut app);
        assert!(
            !app.world().resource::<RunningEvent>().active(),
            "format {version}"
        );
        resume(&mut app);
        assert!(!switch_on(&app, 9998));
        assert!(!app.world().resource::<EventSaveRequest>().0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn invalid_foreground_frames_do_not_mutate_the_live_session_or_the_file() {
    let (mut app, path) = app("invalid");
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, vec![cmd(11910, 0, vec![])]);
    app.update();
    let base = app.world().resource::<RunningEvent>().frame.clone();
    app.update();
    let original = std::fs::read_to_string(&path).unwrap();
    let prefix = original.split("    foreground:").next().unwrap();
    for variant in 0..11 {
        let mut frame = base.clone();
        match variant {
            0 => frame.ip = usize::MAX,
            1 => frame.wait = f32::NAN,
            2 => frame.wait = f32::INFINITY,
            3 => frame.active = false,
            4 => frame.parallel = true,
            5 => frame.choice_pending = true,
            6 => frame.input_pending = true,
            7 => frame.shop_pending = true,
            8 => frame.battle_pending = true,
            9 => frame.call_stack.push(crate::interpreter::frame::CallFrame {
                commands: Vec::new(),
                ip: 1,
                event_id: 0,
            }),
            _ => {
                frame.call_stack = vec![
                    crate::interpreter::frame::CallFrame {
                        commands: Vec::new(),
                        ip: 0,
                        event_id: 0,
                    };
                    crate::interpreter::frame::MAX_CALL_DEPTH + 1
                ]
            }
        }
        let content = format!(
            "{prefix}    foreground: Some((frame: {})),\n)",
            ron::to_string(&frame).unwrap()
        );
        std::fs::write(&path, &content).unwrap();
        app.world_mut().resource_mut::<Switches>().set(9998, true);
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(
            app.world().resource::<LoadOutcome>().0,
            Some(false),
            "variant {variant}"
        );
        assert!(app.world().resource::<PendingTeleport>().0.is_none());
        assert!(switch_on(&app, 9998));
        assert!(!app.world().resource::<RunningEvent>().active());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    }
    std::fs::remove_file(path).unwrap();
}
