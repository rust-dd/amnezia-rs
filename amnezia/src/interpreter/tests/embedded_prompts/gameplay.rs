use super::*;

pub(super) fn app() -> App {
    let mut app = interp_app();
    app.add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_resource::<crate::menu::DirectionInput>()
        .insert_resource(HeroName("Ron".into()))
        .add_plugins((
            crate::dialogue::DialoguePlugin,
            crate::choice::ChoicePlugin,
            crate::inputnumber::InputNumberPlugin,
        ))
        .add_systems(
            Update,
            crate::menu::update_directions.in_set(crate::menu::MenuInput),
        );
    app.update();
    app
}

pub(super) fn step(app: &mut App, keys: &[KeyCode]) {
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    *input = default();
    for key in keys {
        input.press(*key);
    }
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame += 1;
    app.update();
}

fn wait_for_prompt(app: &mut App, numeric: bool) {
    for _ in 0..200 {
        let world = app.world();
        let active = if numeric {
            world.resource::<InputNumber>().active()
        } else {
            world.resource::<Choice>().active()
        };
        if active && world.resource::<Dialogue>().prompt_input_ready() {
            return;
        }
        assert!(world.resource::<RunningEvent>().active());
        step(app, &[]);
    }
    panic!("embedded prompt never became ready");
}

#[test]
fn typing_choices_keeps_the_question_and_returns_the_selected_or_cancelled_branch_once() {
    for cancel in [false, true] {
        let mut app = app();
        let mut commands = vec![message("\\S[5]Kérdés", false)];
        commands.extend(choices());
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(1, commands);
        step(&mut app, &[]);
        for _ in 0..8 {
            step(&mut app, &[KeyCode::Enter, KeyCode::ArrowDown]);
            assert!(app.world().resource::<Dialogue>().active);
            assert!(!app.world().resource::<Choice>().active());
            assert!(!switch_on(&app, 40) && !switch_on(&app, 41));
        }
        wait_for_prompt(&mut app, false);
        assert_eq!(
            app.world().resource::<Dialogue>().boxes[0].lines[0],
            "\\S[5]Kérdés"
        );
        assert_eq!(app.world().resource::<Choice>().cursor, 0);
        step(
            &mut app,
            &[if cancel {
                KeyCode::Escape
            } else {
                KeyCode::Enter
            }],
        );
        for _ in 0..3 {
            step(&mut app, &[]);
        }
        assert_eq!(switch_on(&app, 40), !cancel);
        assert_eq!(switch_on(&app, 41), cancel);
        assert!(switch_on(&app, 42));
        assert!(!app.world().resource::<Dialogue>().active);
        assert!(!app.world().resource::<RunningEvent>().active());
        assert_eq!(app.world().resource::<Choice>().result, None);
    }
}

#[test]
fn numeric_input_keeps_all_three_body_lines_and_commits_the_edited_value_once() {
    let mut app = app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            message("Számbekérés", false),
            message("Második sor", true),
            message("Harmadik sor", true),
            cmd(10150, 0, vec![4, 76]),
            switch_cmd(42, 2, 0),
        ],
    );
    step(&mut app, &[]);
    wait_for_prompt(&mut app, true);
    assert_eq!(app.world().resource::<Dialogue>().boxes[0].lines.len(), 3);
    step(&mut app, &[KeyCode::ArrowUp]);
    assert_eq!(app.world().resource::<InputNumber>().value, 1);
    step(&mut app, &[KeyCode::Enter]);
    for _ in 0..5 {
        step(&mut app, &[]);
    }
    assert_eq!(app.world().resource::<Variables>().get(76), 1);
    assert!(switch_on(&app, 42));
    assert!(!app.world().resource::<Dialogue>().active);
    assert!(!app.world().resource::<RunningEvent>().active());
    assert_eq!(app.world().resource::<InputNumber>().result, None);
}

#[test]
fn parallel_work_continues_but_cannot_take_an_embedded_choices_result() {
    let mut app = app();
    let mut commands = vec![message("Kérdés", false)];
    commands.extend(choices());
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, commands),
            map_event(2, 4, vec![switch_cmd(43, 2, 0)]),
        ],
    });
    let mut active_seen = false;
    for _ in 0..100 {
        let before = switch_on(&app, 43);
        step(&mut app, &[]);
        assert_ne!(switch_on(&app, 43), before);
        if app.world().resource::<Choice>().active()
            && app.world().resource::<Dialogue>().prompt_input_ready()
        {
            active_seen = true;
            break;
        }
    }
    assert!(active_seen);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(3, vec![switch_cmd(44, 0, 0)]);
    step(&mut app, &[KeyCode::Escape]);
    for _ in 0..3 {
        step(&mut app, &[]);
    }
    assert!(!switch_on(&app, 40));
    assert!(switch_on(&app, 41));
    assert!(switch_on(&app, 42));
    assert!(switch_on(&app, 44));
    assert_eq!(app.world().resource::<Choice>().result, None);
}

#[test]
fn number_edit_and_confirm_are_blocked_during_the_final_text_wait() {
    let mut app = app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            message("\\>ab", false),
            cmd(10150, 0, vec![4, 76]),
            switch_cmd(42, 0, 0),
        ],
    );
    let mut checked = false;
    for _ in 0..10 {
        let world = app.world();
        if world.resource::<InputNumber>().active()
            && !world.resource::<Dialogue>().prompt_input_ready()
        {
            step(&mut app, &[KeyCode::ArrowUp, KeyCode::Enter]);
            assert!(app.world().resource::<InputNumber>().active());
            assert_eq!(app.world().resource::<InputNumber>().value, 0);
            assert_eq!(app.world().resource::<InputNumber>().result, None);
            checked = true;
        } else {
            step(&mut app, &[]);
        }
    }
    assert!(checked);
    assert!(!switch_on(&app, 42));
    step(&mut app, &[KeyCode::Enter]);
    for _ in 0..3 {
        step(&mut app, &[]);
    }
    assert!(switch_on(&app, 42));
}
