use super::*;
use crate::choice::Choice;
use crate::inputnumber::InputNumber;
use crate::interpreter::RunningEvent;
use crate::state::{Inventory, Party, Switches, Variables};
use crate::timing::{GameFrames, SceneFrames, TimingPlugin};
use crate::transitions::{Kind, Transition, TransitionPlugin};
use crate::world::{MapData, MapEvents};

fn app(fps: u32) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TimingPlugin,
        TransitionPlugin,
        crate::choice::ChoicePlugin,
        crate::inputnumber::InputNumberPlugin,
    ))
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / fps as f64),
    ))
    .insert_resource(MapData::for_test(20, 15))
    .insert_resource(MapEvents { events: Vec::new() })
    .insert_resource(crate::text::HeroName("Ron".into()))
    .init_resource::<Dialogue>()
    .init_resource::<Switches>()
    .init_resource::<Variables>()
    .init_resource::<Party>()
    .init_resource::<Inventory>()
    .init_resource::<RunningEvent>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<crate::menu::DirectionInput>()
    .add_message::<crate::audio::AudioRequest>()
    .add_systems(
        Update,
        (
            crate::menu::update_directions.in_set(crate::menu::MenuInput),
            interact.after(PromptInput),
            typewriter::drive_reveal,
            embedded::update,
        )
            .chain(),
    );
    InputPrompts::register(&mut app);
    app.update();
    app
}

fn open(app: &mut App, raw: &str) {
    app.world_mut()
        .resource_mut::<Dialogue>()
        .open(vec![MessageBox {
            face: None,
            face_index: 0,
            lines: vec![raw.into()],
        }]);
}

fn start_transition(app: &mut App, duration: u32) {
    let frame = app.world().resource::<GameFrames>().frame;
    assert!(app.world_mut().resource_mut::<Transition>().start_for(
        Kind::Fade,
        false,
        frame,
        IVec2::new(160, 120),
        duration,
    ));
}

fn step(app: &mut App, pressed: &[KeyCode]) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    *keys = default();
    for key in pressed {
        keys.press(*key);
    }
    app.update();
}

#[test]
fn reveal_and_pause_arrow_hold_through_the_terminal_async_frame_at_every_fps() {
    for fps in [15, 30, 60, 120, 144] {
        for raw in ["abcdefghijklmnop", "ab\\|c", "ab\\!"] {
            let mut app = app(fps);
            open(&mut app, raw);
            app.update();
            let mut reference = Typewriter::new(raw, "Ron", &Variables::default());
            reference.tick();
            let scene = app.world().resource::<SceneFrames>().frame;
            start_transition(&mut app, 35);
            for render in 0..fps {
                if !app.world().resource::<Transition>().busy() {
                    break;
                }
                app.update();
                let dialogue = app.world().resource::<Dialogue>();
                let reveal = dialogue.reveal.as_ref().unwrap();
                assert_eq!(app.world().resource::<SceneFrames>().frame, scene);
                assert_eq!(
                    reveal.text(),
                    reference.text(),
                    "{fps} FPS, render {render}"
                );
                assert_eq!(reveal.arrow_visible(), reference.arrow_visible());
            }
            assert!(!app.world().resource::<Transition>().busy());
            for _ in 0..fps {
                let before = app.world().resource::<GameFrames>().frame;
                app.update();
                let ticks = app
                    .world()
                    .resource::<GameFrames>()
                    .frame
                    .wrapping_sub(before);
                for _ in 0..ticks {
                    reference.tick();
                }
                let dialogue = app.world().resource::<Dialogue>();
                let reveal = dialogue.reveal.as_ref().unwrap();
                assert_eq!(reveal.text(), reference.text());
                assert_eq!(reveal.arrow_visible(), reference.arrow_visible());
                assert_eq!(reveal.is_complete(), reference.is_complete());
            }
        }
    }
}

#[test]
fn completed_messages_ignore_decision_and_cancel_on_the_last_transition_frame() {
    for key in [KeyCode::Enter, KeyCode::Space, KeyCode::Escape] {
        let mut app = app(60);
        open(&mut app, "ab");
        for _ in 0..8 {
            step(&mut app, &[]);
        }
        start_transition(&mut app, 6);
        for _ in 0..5 {
            step(&mut app, &[]);
        }
        step(&mut app, &[key]);
        assert!(!app.world().resource::<Transition>().busy());
        assert!(app.world().resource::<Dialogue>().active, "{key:?}");
        assert!(
            app.world()
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(key)
        );
        step(&mut app, &[]);
        assert!(app.world().resource::<Dialogue>().active);
        step(&mut app, &[key]);
        assert!(!app.world().resource::<Dialogue>().active);
    }
}

#[test]
fn an_explicit_key_wait_is_not_released_by_the_terminal_transition_key() {
    for key in [KeyCode::Enter, KeyCode::Space, KeyCode::Escape] {
        let mut app = app(60);
        open(&mut app, "ab\\!cd");
        for _ in 0..8 {
            step(&mut app, &[]);
        }
        start_transition(&mut app, 6);
        for _ in 0..5 {
            step(&mut app, &[]);
        }
        step(&mut app, &[key]);
        let dialogue = app.world().resource::<Dialogue>();
        assert!(dialogue.reveal.as_ref().unwrap().waiting_for_key());
        step(&mut app, &[]);
        step(&mut app, &[key]);
        let dialogue = app.world().resource::<Dialogue>();
        assert!(dialogue.active);
        assert!(!dialogue.reveal.as_ref().unwrap().waiting_for_key());
        assert_eq!(dialogue.reveal.as_ref().unwrap().text(), "abc");
        step(&mut app, &[]);
        let dialogue = app.world().resource::<Dialogue>();
        assert_eq!(dialogue.reveal.as_ref().unwrap().text(), "abcd");
    }
}

#[test]
fn regular_battles_allow_prompts_but_shop_title_and_gameover_scenes_do_not() {
    for numeric in [false, true] {
        for scene in 0..4 {
            let mut app = app(60);
            if numeric {
                app.world_mut().resource_mut::<InputNumber>().open(2, 76);
            } else {
                app.world_mut().resource_mut::<Choice>().open(
                    vec!["Első".into(), "Második".into()],
                    0,
                    0,
                );
            }
            match scene {
                0 => {
                    app.insert_resource(crate::battle::BattleActive(true));
                }
                1 => {
                    app.insert_resource(crate::shop::ShopOpen(true));
                }
                2 => {
                    app.insert_resource(crate::title::TitleActive(true));
                }
                _ => {
                    app.insert_resource(crate::gameover::GameOverActive(true));
                }
            }
            step(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]);
            if numeric {
                let number = app.world().resource::<InputNumber>();
                assert_eq!(number.active(), scene != 0);
                assert_eq!(number.result, (scene == 0).then_some(9));
            } else {
                let choice = app.world().resource::<Choice>();
                assert_eq!(choice.active(), scene != 0);
                assert_eq!(choice.result, (scene == 0).then_some(1));
            }
        }
    }
}

#[test]
fn choice_and_number_input_discard_the_terminal_transition_keys() {
    for numeric in [false, true] {
        let mut app = app(60);
        if numeric {
            app.world_mut().resource_mut::<InputNumber>().open(2, 76);
        } else {
            app.world_mut()
                .resource_mut::<Dialogue>()
                .open_prompt(MessagePrompt::Choice {
                    labels: vec!["Első".into(), "Második".into()],
                    indent: 0,
                    cancel: 2,
                });
        }
        for _ in 0..30 {
            step(&mut app, &[]);
        }
        start_transition(&mut app, 6);
        for _ in 0..5 {
            step(&mut app, &[]);
        }
        step(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]);
        assert!(!app.world().resource::<Transition>().busy());
        if numeric {
            let number = app.world().resource::<InputNumber>();
            assert!(number.active());
            assert_eq!(number.value, 0);
            assert_eq!(number.result, None);
        } else {
            let choice = app.world().resource::<Choice>();
            assert!(choice.active());
            assert_eq!(choice.cursor, 0);
            assert_eq!(choice.result, None);
        }
        step(&mut app, &[]);
        step(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]);
        if numeric {
            assert_eq!(app.world().resource::<InputNumber>().result, Some(9));
        } else {
            assert_eq!(app.world().resource::<Choice>().result, Some(1));
        }
    }
}

#[test]
fn a_new_message_waits_for_a_teleport_handoff_consumed_earlier_in_the_frame() {
    let mut app = app(60);
    app.init_resource::<crate::teleport::PendingTeleport>()
        .add_systems(
            PreUpdate,
            (|mut pending: ResMut<crate::teleport::PendingTeleport>| pending.0 = None)
                .after(crate::timing::FrameClockSet),
        );
    app.world_mut()
        .resource_mut::<crate::teleport::PendingTeleport>()
        .reload(3, 0, 0);
    open(&mut app, "abcdef");
    app.update();
    assert!(app.world().resource::<Dialogue>().reveal.is_none());
    app.update();
    let dialogue = app.world().resource::<Dialogue>();
    assert_eq!(dialogue.reveal.as_ref().unwrap().text(), "ab");
}
