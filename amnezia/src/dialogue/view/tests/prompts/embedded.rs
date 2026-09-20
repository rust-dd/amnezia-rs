use super::*;
use crate::dialogue::MessagePrompt;

fn show(app: &mut App, numeric: bool, face: bool) {
    let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
    dialogue.open(vec![crate::events::MessageBox {
        face: face.then(|| "Ron".into()),
        face_index: 6,
        lines: vec!["Ron".into(), "Kérdés".into()],
    }]);
    assert!(dialogue.append_prompt(if numeric {
        MessagePrompt::Number {
            digits: 4,
            variable: 76,
        }
    } else {
        MessagePrompt::Choice {
            labels: vec!["Igen".into(), "Nem".into()],
            indent: 0,
            cancel: 2,
        }
    }));
    let mut reveal = crate::dialogue::Typewriter::new(
        &dialogue.boxes[0].lines.join("\n"),
        "Ron",
        &crate::state::Variables::default(),
    );
    reveal.expect_prompt();
    for _ in 0..100 {
        reveal.tick();
    }
    dialogue.reveal = Some(reveal);
}

#[test]
fn embedded_choices_keep_body_columns_and_indent_only_the_choice_rows() {
    let mut app = prompt_app();
    for face in [false, true] {
        show(&mut app, false, face);
        app.world_mut()
            .resource_mut::<Choice>()
            .open(vec!["Igen".into(), "Nem".into()], 0, 2);
        app.world_mut().resource_mut::<Choice>().cursor = 1;
        app.update();
        let left = if face { 72 } else { 0 };
        assert_eq!(
            text(&mut app).runs,
            vec![
                Run::new("Ron", left, 2, DEFAULT),
                Run::new("Kérdés", left, 18, DEFAULT),
                Run::new("Igen", left + 12, 34, DEFAULT),
                Run::new("Nem", left + 12, 50, DEFAULT),
            ]
        );
        assert_eq!(cursor(&mut app).0.top, Val::Px(168.0));
    }
}

#[test]
fn embedded_number_digits_and_cursor_follow_the_body_without_erasing_it() {
    let mut app = prompt_app();
    for face in [false, true] {
        show(&mut app, true, face);
        app.world_mut().resource_mut::<InputNumber>().open(4, 76);
        app.update();
        let left = if face { 72 } else { 0 };
        assert_eq!(
            text(&mut app).runs,
            vec![
                Run::new("Ron", left, 2, DEFAULT),
                Run::new("Kérdés", left, 18, DEFAULT),
                Run::new("0", left + 12, 34, DEFAULT),
                Run::new("0", left + 24, 34, DEFAULT),
                Run::new("0", left + 36, 34, DEFAULT),
                Run::new("0", left + 48, 34, DEFAULT),
            ]
        );
        assert_eq!(cursor(&mut app).0.top, Val::Px(120.0));
    }
}

#[test]
fn an_embedded_cursor_does_not_draw_before_the_choice_text_is_revealed() {
    let mut app = prompt_app();
    show(&mut app, false, true);
    let mut reveal = crate::dialogue::Typewriter::new(
        "Ron\nKérdés\nIgen\nNem",
        "Ron",
        &crate::state::Variables::default(),
    );
    reveal.expect_prompt();
    reveal.tick();
    app.world_mut().resource_mut::<Dialogue>().reveal = Some(reveal);
    app.update();
    assert_eq!(cursor(&mut app).1, Visibility::Hidden);
    assert_eq!(text(&mut app).runs, vec![Run::new("Ro", 72, 2, DEFAULT)]);
}
