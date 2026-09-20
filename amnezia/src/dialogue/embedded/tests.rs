use super::*;
use crate::dialogue::typewriter::{Typewriter, drive_reveal};
use crate::events::MessageBox;
use crate::state::Variables;
use crate::timing::GameFrames;

fn app(body: &[&str], numeric: bool) -> App {
    let mut app = App::new();
    app.init_resource::<Dialogue>()
        .init_resource::<Choice>()
        .init_resource::<InputNumber>()
        .init_resource::<Variables>()
        .init_resource::<GameFrames>()
        .insert_resource(crate::text::HeroName("Ron".into()))
        .add_systems(Update, (drive_reveal, update).chain());
    let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
    dialogue.open(vec![MessageBox {
        face: Some("Ron".into()),
        face_index: 6,
        lines: body.iter().map(|line| line.to_string()).collect(),
    }]);
    assert!(dialogue.append_prompt(if numeric {
        MessagePrompt::Number {
            digits: 4,
            variable: 76,
        }
    } else {
        MessagePrompt::Choice {
            labels: vec!["cd".into(), "ef".into()],
            indent: 1,
            cancel: 2,
        }
    }));
    app
}

fn tick(app: &mut App) {
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    app.update();
}

#[test]
fn embedded_options_type_as_message_lines_without_a_final_pause_arrow() {
    let mut app = app(&["ab"], false);
    for (age, expected) in ["ab", "ab\ncd", "ab\ncd\nef", "ab\ncd\nef", "ab\ncd\nef"]
        .into_iter()
        .enumerate()
    {
        tick(&mut app);
        let dialogue = app.world().resource::<Dialogue>();
        let reveal = dialogue.reveal.as_ref().unwrap();
        assert_eq!(reveal.text(), expected, "tick {}", age + 1);
        assert!(!reveal.arrow_visible());
        assert_eq!(app.world().resource::<Choice>().active(), age == 4);
        assert_eq!(dialogue.prompt_input_ready(), age == 4);
    }
    assert!(app.world().resource::<Dialogue>().active);
}

#[test]
fn a_numeric_cursor_appears_at_page_end_but_cannot_accept_during_the_final_wait() {
    let mut app = app(&["\\>ab"], true);
    for age in 1..=3 {
        tick(&mut app);
        let dialogue = app.world().resource::<Dialogue>();
        assert!(app.world().resource::<InputNumber>().active());
        assert_eq!(dialogue.prompt_input_ready(), age == 3);
        assert!(!dialogue.reveal.as_ref().unwrap().arrow_visible());
    }
}

#[test]
fn explicit_key_waits_still_work_and_auto_close_codes_do_not_skip_a_prompt() {
    let mut app = app(&["ab\\!c\\^"], true);
    for _ in 0..20 {
        tick(&mut app);
    }
    {
        let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
        let reveal = dialogue.reveal.as_mut().unwrap();
        assert_eq!(reveal.text(), "ab");
        assert!(reveal.waiting_for_key());
        reveal.resume();
    }
    for _ in 0..20 {
        tick(&mut app);
    }
    let dialogue = app.world().resource::<Dialogue>();
    assert!(dialogue.active && dialogue.prompt_input_ready());
    assert!(app.world().resource::<InputNumber>().active());
    assert_eq!(dialogue.reveal.as_ref().unwrap().text(), "abc");
}

#[test]
fn prompt_reveal_tracks_the_same_logical_ticks_at_all_render_rates_and_after_pauses() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = app(&["\\S[5]ab"], false);
        app.update();
        let mut reference = Typewriter::new("\\S[5]ab\ncd\nef", "Ron", &Variables::default());
        reference.expect_prompt();
        reference.tick();
        for render in 0..fps * 2 {
            let paused = (fps / 2..fps).contains(&render);
            app.insert_resource(crate::menu::MenuOpen(paused));
            let before = app.world().resource::<GameFrames>().frame;
            app.world_mut()
                .resource_mut::<GameFrames>()
                .advance(1.0 / fps as f64);
            let after = app.world().resource::<GameFrames>().frame;
            if !paused {
                for _ in before..after {
                    reference.tick();
                }
            }
            app.update();
            let dialogue = app.world().resource::<Dialogue>();
            assert_eq!(
                dialogue.reveal.as_ref().unwrap().text(),
                reference.text(),
                "{fps} FPS render {render}"
            );
            assert_eq!(dialogue.prompt_input_ready(), reference.is_complete());
        }
        assert!(app.world().resource::<Choice>().active());
    }
}
