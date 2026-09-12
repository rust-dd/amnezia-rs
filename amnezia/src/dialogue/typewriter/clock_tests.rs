use super::*;
use crate::dialogue::Dialogue;
use crate::timing::GameFrames;

fn app(raw: &str) -> App {
    let mut app = App::new();
    app.init_resource::<Dialogue>()
        .init_resource::<Variables>()
        .init_resource::<GameFrames>()
        .insert_resource(crate::text::HeroName("Ron".into()))
        .add_systems(Update, drive_reveal);
    app.world_mut()
        .resource_mut::<Dialogue>()
        .open(vec![crate::events::MessageBox {
            face: None,
            face_index: 0,
            lines: vec![raw.into()],
        }]);
    app.update();
    app
}

fn text(app: &App) -> &str {
    app.world()
        .resource::<Dialogue>()
        .reveal
        .as_ref()
        .unwrap()
        .text()
}

#[test]
fn rendering_without_a_new_logical_tick_does_not_reveal_more_text() {
    let mut app = app("abcdef");
    let before = text(&app).to_string();
    for _ in 0..8 {
        app.update();
    }
    assert_eq!(text(&app), before);
}

#[test]
fn a_menu_holds_the_reveal_and_discards_paused_ticks() {
    let mut app = app("abcdefghijklmnop");
    let before = text(&app).to_string();
    app.insert_resource(crate::menu::MenuOpen(true));
    for _ in 0..20 {
        app.world_mut().resource_mut::<GameFrames>().frame += 1;
        app.update();
    }
    assert_eq!(text(&app), before);
    app.world_mut().resource_mut::<crate::menu::MenuOpen>().0 = false;
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    app.update();
    assert_eq!(text(&app), "abcd");
}

#[test]
fn bevy_reveal_matches_the_same_logical_timeline_at_low_and_high_fps() {
    let raw = "Árvíztűrő\n\\S[5]abcdefghijklmnop\\|utána vége";
    for fps in [15, 30, 60, 120, 144] {
        let mut app = app(raw);
        let mut reference = Typewriter::new(raw, "Ron", &Variables::default());
        reference.tick();
        for _ in 0..fps * 5 {
            let before = app.world().resource::<GameFrames>().frame;
            app.world_mut()
                .resource_mut::<GameFrames>()
                .advance(1.0 / fps as f64);
            app.update();
            let after = app.world().resource::<GameFrames>().frame;
            for _ in before..after {
                reference.tick();
            }
            let actual = app.world().resource::<Dialogue>().reveal.as_ref().unwrap();
            assert_eq!(
                (
                    actual.text(),
                    actual.is_complete(),
                    actual.wait,
                    actual.arrow_visible()
                ),
                (
                    reference.text(),
                    reference.is_complete(),
                    reference.wait,
                    reference.arrow_visible()
                ),
                "{fps} FPS at {after}"
            );
        }
        assert!(reference.is_complete());
    }
}

#[test]
fn other_scenes_pause_text_but_a_regular_battle_does_not() {
    for mode in 0..4 {
        let mut app = app("abcdefghijklmnop");
        match mode {
            0 => {
                app.insert_resource(crate::shop::ShopOpen(true));
            }
            1 => {
                app.insert_resource(crate::title::TitleActive(true));
            }
            2 => {
                app.insert_resource(crate::gameover::GameOverActive(true));
            }
            _ => {
                let mut transition = crate::transitions::Transition::default();
                transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
                app.insert_resource(transition);
            }
        }
        for frame in 1..=120 {
            app.world_mut().resource_mut::<GameFrames>().frame = frame;
            app.update();
        }
        assert_eq!(text(&app), "ab");
    }
    let mut app = app("abcdefghijklmnop");
    app.insert_resource(crate::battle::BattleActive(true));
    app.world_mut().resource_mut::<GameFrames>().frame = 1;
    app.update();
    assert_eq!(text(&app), "abcd");
}

#[test]
fn wrapping_the_global_clock_does_not_skip_text_or_restart_the_reveal() {
    let mut app = app("abcdefghijklmnop");
    app.world_mut()
        .resource_mut::<Dialogue>()
        .reveal
        .as_mut()
        .unwrap()
        .last_frame = Some(u32::MAX - 1);
    for frame in [u32::MAX, 0] {
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
    }
    assert_eq!(text(&app), "abcdef");
}
