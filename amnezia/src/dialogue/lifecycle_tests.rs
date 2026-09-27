use super::*;
use crate::state::Variables;
use crate::timing::GameFrames;

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<Dialogue>()
        .init_resource::<Variables>()
        .init_resource::<GameFrames>()
        .insert_resource(crate::text::HeroName("Ron".into()))
        .add_systems(Update, typewriter::drive_reveal);
    app.world_mut()
        .resource_mut::<Dialogue>()
        .open(vec![MessageBox {
            face: Some("Ron".into()),
            face_index: 0,
            lines: vec!["abcdef".into()],
        }]);
    app
}

#[test]
fn a_map_message_reveals_nothing_until_the_seventh_opening_tick() {
    let mut app = app();
    for frame in 0..7 {
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
        let dialogue = app.world().resource::<Dialogue>();
        assert!(dialogue.active);
        assert!(
            dialogue
                .reveal
                .as_ref()
                .is_none_or(|reveal| reveal.text().is_empty()),
            "opening frame {frame}"
        );
    }
    app.world_mut().resource_mut::<GameFrames>().frame = 7;
    app.update();
    assert_eq!(
        app.world()
            .resource::<Dialogue>()
            .reveal
            .as_ref()
            .unwrap()
            .text(),
        "ab"
    );
}

#[test]
fn closing_owns_all_seven_frames_and_hides_contents_before_releasing_the_map() {
    let mut app = app();
    app.update();
    app.world_mut().resource_mut::<GameFrames>().frame = 7;
    app.update();
    {
        let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
        dialogue.lifecycle.gold.open(false);
        dialogue.finish(7);
        assert!(!dialogue.active);
        assert!(dialogue.busy());
        assert!(dialogue.allows_next(true));
        assert!(!dialogue.allows_next(false));
        assert!(dialogue.reveal.is_none());
        assert_eq!(dialogue.lifecycle.message.half_height(80), 34);
        assert_eq!(dialogue.lifecycle.gold.half_height(32), 13);
    }
    app.update();
    for frame in 8..=13 {
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
        assert!(app.world().resource::<Dialogue>().busy());
        assert!(!app.world().resource::<Dialogue>().allows_next(true));
    }
    assert!(
        !app.world()
            .resource::<Dialogue>()
            .lifecycle
            .message
            .visible()
    );
    app.world_mut().resource_mut::<GameFrames>().frame = 14;
    app.update();
    assert!(!app.world().resource::<Dialogue>().busy());
}

#[test]
fn opening_uses_logical_ticks_at_every_render_rate_without_charging_prior_time() {
    for fps in [15, 30, 60, 120, 144, 240] {
        let mut app = app();
        app.world_mut().resource_mut::<GameFrames>().frame = 600;
        app.update();
        let mut expected = Typewriter::new("abcdef", "Ron", &Variables::default());
        for _ in 0..fps {
            let previous = app.world().resource::<GameFrames>().frame - 600;
            app.world_mut()
                .resource_mut::<GameFrames>()
                .advance(1.0 / f64::from(fps));
            app.update();
            let age = app.world().resource::<GameFrames>().frame - 600;
            for frame in previous + 1..=age {
                if frame >= 7 {
                    expected.tick();
                }
            }
            let dialogue = app.world().resource::<Dialogue>();
            assert_eq!(
                dialogue.lifecycle.message.ready(),
                age >= 7,
                "{fps} FPS, {age}"
            );
            assert_eq!(
                dialogue.reveal.as_ref().map_or("", Typewriter::text),
                expected.text()
            );
        }
    }
}

#[test]
fn interrupted_parallel_messages_close_message_and_gold_in_seven_steps() {
    let mut app = app();
    app.update();
    app.world_mut().resource_mut::<GameFrames>().frame = 7;
    app.update();
    {
        let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
        assert!(!dialogue.finish_parallel(7));
        assert!(dialogue.active);
        dialogue.from_foreground = false;
        dialogue.open_gold();
        assert!(dialogue.finish_parallel(7));
        assert!(!dialogue.active);
        assert!(dialogue.boxes.is_empty());
        assert!(dialogue.reveal.is_none());
    }
    for (index, (message, gold)) in [
        (34, 13),
        (28, 11),
        (22, 9),
        (17, 6),
        (11, 4),
        (5, 2),
        (0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        app.world_mut().resource_mut::<GameFrames>().frame = 7 + index as u32;
        app.update();
        let dialogue = app.world().resource::<Dialogue>();
        assert!(dialogue.busy());
        assert_eq!(dialogue.lifecycle.message.half_height(80), message);
        assert_eq!(dialogue.lifecycle.gold.half_height(32), gold);
    }
    app.world_mut().resource_mut::<GameFrames>().frame = 14;
    app.update();
    assert!(!app.world().resource::<Dialogue>().busy());
}

#[test]
fn a_transfer_restarts_closing_after_either_message_owner_has_finished() {
    for foreground in [false, true] {
        let mut app = app();
        app.update();
        app.world_mut().resource_mut::<GameFrames>().frame = 7;
        app.update();
        {
            let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
            dialogue.from_foreground = foreground;
            dialogue.open_gold();
            dialogue.finish(7);
        }
        app.world_mut().resource_mut::<GameFrames>().frame = 9;
        app.update();
        let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
        assert_eq!(dialogue.lifecycle.message.half_height(80), 22);
        assert!(!dialogue.finish_parallel(9));
        assert_eq!(dialogue.lifecycle.message.half_height(80), 34);
        assert_eq!(dialogue.lifecycle.gold.half_height(32), 13);
    }
}

#[test]
fn battle_messages_have_no_window_animation() {
    let mut app = app();
    app.insert_resource(crate::battle::BattleActive(true));
    app.update();
    let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
    assert!(dialogue.lifecycle.message.ready());
    assert_eq!(dialogue.reveal.as_ref().unwrap().text(), "ab");
    dialogue.finish(0);
    assert!(!dialogue.lifecycle.message.visible());
}

#[test]
fn an_async_wait_freezes_both_windows_and_does_not_replay_elapsed_ticks() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<Dialogue>()
        .lifecycle
        .gold
        .open(true);
    app.update();
    app.world_mut().resource_mut::<GameFrames>().frame = 3;
    app.update();
    app.insert_resource(crate::timing::SceneWait(true));
    for frame in 4..=120 {
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
        let dialogue = app.world().resource::<Dialogue>();
        assert_eq!(dialogue.lifecycle.message.half_height(80), 17);
        assert_eq!(dialogue.lifecycle.gold.half_height(32), 6);
        assert!(dialogue.reveal.is_none());
    }
    app.world_mut().resource_mut::<crate::timing::SceneWait>().0 = false;
    app.world_mut().resource_mut::<GameFrames>().frame = 121;
    app.update();
    let dialogue = app.world().resource::<Dialogue>();
    assert_eq!(dialogue.lifecycle.message.half_height(80), 22);
    assert_eq!(dialogue.lifecycle.gold.half_height(32), 9);
}

#[test]
fn a_five_line_message_clears_its_page_for_one_tick_without_reopening() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<Dialogue>()
        .open(vec![MessageBox {
            face: Some("Ron".into()),
            face_index: 2,
            lines: vec!["a".into(), "b".into(), "c".into(), "d".into(), "ef".into()],
        }]);
    app.update();
    app.world_mut().resource_mut::<GameFrames>().frame = 40;
    app.update();
    {
        let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
        assert_eq!(dialogue.boxes.len(), 2);
        assert_eq!(dialogue.boxes[0].lines, ["a", "b", "c", "d"]);
        assert_eq!(dialogue.boxes[1].lines, ["ef"]);
        assert_eq!(dialogue.boxes[1].face_index, 2);
        assert!(dialogue.reveal.as_ref().unwrap().is_complete());
        dialogue.advance(41);
    }
    app.world_mut().resource_mut::<GameFrames>().frame = 41;
    app.update();
    assert!(app.world().resource::<Dialogue>().reveal.is_none());
    assert!(app.world().resource::<Dialogue>().lifecycle.message.ready());
    app.world_mut().resource_mut::<GameFrames>().frame = 42;
    app.update();
    assert_eq!(
        app.world()
            .resource::<Dialogue>()
            .reveal
            .as_ref()
            .unwrap()
            .text(),
        "ef"
    );
}

#[test]
fn an_embedded_prompt_closes_on_the_same_clock_as_a_normal_message() {
    let mut app = app();
    app.init_resource::<crate::choice::Choice>()
        .init_resource::<crate::inputnumber::InputNumber>()
        .add_systems(Update, embedded::update.after(typewriter::drive_reveal));
    app.world_mut().resource_mut::<Dialogue>().open_number(
        3,
        10,
        &mut crate::inputnumber::InputNumber::default(),
    );
    app.world_mut()
        .resource_mut::<crate::inputnumber::InputNumber>()
        .open(3, 10);
    app.update();
    app.world_mut().resource_mut::<GameFrames>().frame = 7;
    app.update();
    assert!(app.world().resource::<Dialogue>().prompt_input_ready());
    app.world_mut()
        .resource_mut::<crate::inputnumber::InputNumber>()
        .active = false;
    app.update();
    assert_eq!(
        app.world()
            .resource::<Dialogue>()
            .lifecycle
            .message
            .half_height(80),
        34
    );
    for (frame, half) in [(8, 28), (9, 22), (10, 17), (11, 11), (12, 5), (13, 0)] {
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
        assert_eq!(
            app.world()
                .resource::<Dialogue>()
                .lifecycle
                .message
                .half_height(80),
            half
        );
        assert!(app.world().resource::<Dialogue>().busy());
    }
    app.world_mut().resource_mut::<GameFrames>().frame = 14;
    app.update();
    assert!(!app.world().resource::<Dialogue>().busy());
}
