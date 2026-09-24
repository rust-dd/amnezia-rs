use super::*;
use crate::timing::GameFrames;
use crate::transitions::Transition;

fn app() -> App {
    let mut app = flow_app();
    app.insert_resource(crate::save::SaveLocation(std::env::temp_dir().join(
        format!(
            "amnezia_title_repetition_absent_{}/slot1.ron",
            std::process::id()
        ),
    )));
    let mut sounds = SystemSounds::default();
    sounds.cursor.name = "CURSOR".into();
    sounds.decision.name = "DECISION".into();
    sounds.buzzer.name = "BUZZER".into();
    app.insert_resource(sounds);
    frame(&mut app, 0);
    frame(&mut app, 35);
    app.world_mut().resource_mut::<TitleState>().cursor = NEW_GAME;
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .clear();
    app
}

fn sounds(app: &mut App) -> Vec<String> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .filter_map(|request| {
            if let AudioRequest::Sound { name, .. } = request {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

fn step(app: &mut App) -> Vec<String> {
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    sounds(app)
}

fn press(app: &mut App, pressed: &[KeyCode]) -> Vec<String> {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    for &key in pressed {
        keys.press(key);
    }
    step(app)
}

#[test]
fn held_title_arrows_repeat_at_the_original_phase_at_every_frame_rate() {
    for fps in [15, 30, 60, 144] {
        let mut app = app();
        let mut expected = GameFrames::default();
        let mut total = 0;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        for render in 0..fps {
            expected.advance(1.0 / fps as f64);
            frame(&mut app, 35 + expected.frame);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
            let actual = sounds(&mut app);
            assert!(actual.iter().all(|name| name == "CURSOR"));
            total += actual.len();
            let wanted = 1 + if expected.frame < 24 {
                0
            } else {
                (expected.frame - 24) / 4 + 1
            };
            assert_eq!(total, wanted as usize, "{fps} FPS, render {render}");
            assert_eq!(app.world().resource::<TitleState>().cursor, total % 3);
        }
    }
}

#[test]
fn title_preserves_global_hold_phase_during_an_asynchronous_fade() {
    let mut app = app();
    app.world_mut().resource_mut::<Transition>().start_for(
        crate::transitions::Kind::Fade,
        false,
        35,
        IVec2::ZERO,
        1000,
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    for _ in 0..23 {
        assert!(step(&mut app).is_empty());
    }
    assert_eq!(app.world().resource::<TitleState>().cursor, 0);
    app.world_mut().resource_mut::<Transition>().clear();
    assert_eq!(step(&mut app), ["CURSOR"]);
    assert_eq!(app.world().resource::<TitleState>().cursor, 1);
    for _ in 0..3 {
        assert!(step(&mut app).is_empty());
    }
    assert_eq!(step(&mut app), ["CURSOR"]);
    assert_eq!(app.world().resource::<TitleState>().cursor, 2);
}

#[test]
fn title_pages_and_simultaneous_directions_keep_original_order_and_sounds() {
    let mut app = app();
    assert_eq!(press(&mut app, &[KeyCode::PageDown]), ["CURSOR"]);
    assert_eq!(app.world().resource::<TitleState>().cursor, SHUTDOWN);
    assert!(press(&mut app, &[KeyCode::PageDown]).is_empty());
    assert_eq!(press(&mut app, &[KeyCode::PageUp]), ["CURSOR"]);
    assert_eq!(app.world().resource::<TitleState>().cursor, NEW_GAME);
    assert!(
        press(
            &mut app,
            &[KeyCode::PageUp, KeyCode::ArrowRight, KeyCode::ArrowLeft]
        )
        .is_empty()
    );
    assert_eq!(
        press(
            &mut app,
            &[
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
                KeyCode::PageDown,
                KeyCode::PageUp
            ]
        ),
        ["CURSOR"; 4]
    );
    assert_eq!(app.world().resource::<TitleState>().cursor, NEW_GAME);
    assert_eq!(
        press(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]),
        ["CURSOR", "BUZZER"]
    );
    assert_eq!(app.world().resource::<TitleState>().stage, Stage::Ready);
    assert_eq!(
        press(
            &mut app,
            &[KeyCode::PageUp, KeyCode::Enter, KeyCode::Escape]
        ),
        ["CURSOR", "DECISION"]
    );
    assert_eq!(
        app.world().resource::<TitleState>().stage,
        Stage::Leaving(TitleAction::NewGame)
    );
}

#[test]
fn rendering_without_ticks_never_repeats_a_held_title_key() {
    let mut app = app();
    assert_eq!(press(&mut app, &[KeyCode::ArrowUp]), ["CURSOR"]);
    for _ in 0..80 {
        app.update();
        assert!(sounds(&mut app).is_empty());
        assert_eq!(app.world().resource::<TitleState>().cursor, SHUTDOWN);
    }
    for _ in 0..22 {
        assert!(step(&mut app).is_empty());
    }
    assert_eq!(step(&mut app), ["CURSOR"]);
    assert_eq!(app.world().resource::<TitleState>().cursor, CONTINUE);
}
