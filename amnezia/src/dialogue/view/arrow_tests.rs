use super::*;
use crate::dialogue::typewriter::drive_reveal;
use crate::state::Variables;
use crate::timing::GameFrames;

fn app(idle: u32) -> App {
    let mut app = App::new();
    app.init_resource::<Dialogue>()
        .init_resource::<GameFrames>()
        .init_resource::<Variables>()
        .insert_resource(crate::text::HeroName("Ron".into()))
        .add_systems(Update, (drive_reveal, render_reveal).chain());
    app.world_mut().spawn((DialogueArrow, Visibility::Hidden));
    for _ in 0..idle {
        app.update();
    }
    app.world_mut()
        .resource_mut::<Dialogue>()
        .open(vec![crate::events::MessageBox {
            face: None,
            face_index: 0,
            lines: vec!["Ron".into()],
        }]);
    app.update();
    for frame in 1..=3 {
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
    }
    assert!(
        app.world()
            .resource::<Dialogue>()
            .reveal
            .as_ref()
            .unwrap()
            .is_complete()
    );
    app
}

fn visible(app: &mut App) -> bool {
    let world = app.world_mut();
    *world
        .query_filtered::<&Visibility, With<DialogueArrow>>()
        .single(world)
        .unwrap()
        == Visibility::Visible
}

fn advance(app: &mut App, ticks: u32) {
    let mut clock = app.world_mut().resource_mut::<GameFrames>();
    clock.frame = clock.frame.wrapping_add(ticks);
    app.update();
}

fn reopen(app: &mut App, raw: &str) {
    app.world_mut()
        .resource_mut::<Dialogue>()
        .open(vec![crate::events::MessageBox {
            face: None,
            face_index: 0,
            lines: vec![raw.into()],
        }]);
    app.update();
}

#[test]
fn the_arrow_starts_at_page_pause_even_before_the_final_wait_allows_input() {
    let mut app = app(0);
    reopen(&mut app, "");
    assert!(visible(&mut app));
    assert!(
        !app.world()
            .resource::<Dialogue>()
            .reveal
            .as_ref()
            .unwrap()
            .is_complete()
    );
    advance(&mut app, 2);
    assert!(
        app.world()
            .resource::<Dialogue>()
            .reveal
            .as_ref()
            .unwrap()
            .is_complete()
    );
    advance(&mut app, 17);
    assert!(visible(&mut app));
    advance(&mut app, 1);
    assert!(!visible(&mut app));
}

#[test]
fn releasing_and_reentering_a_text_pause_resets_the_blink_cycle() {
    let mut app = app(0);
    reopen(&mut app, "a\\!b\\!c");
    assert!(visible(&mut app));
    advance(&mut app, 20);
    assert!(!visible(&mut app));
    app.world_mut()
        .resource_mut::<Dialogue>()
        .reveal
        .as_mut()
        .unwrap()
        .resume();
    app.update();
    assert!(!visible(&mut app));
    advance(&mut app, 1);
    assert!(visible(&mut app));
    advance(&mut app, 20);
    assert!(!visible(&mut app));
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert!(!visible(&mut app));
    reopen(&mut app, "Ron");
    advance(&mut app, 3);
    assert!(visible(&mut app));
}

#[test]
fn automatically_closing_pages_never_show_a_continue_arrow() {
    for raw in ["a\\^", "\\>ab\\^", "ab\\|c\\^"] {
        let mut app = app(0);
        reopen(&mut app, raw);
        for _ in 0..150 {
            assert!(!visible(&mut app));
            advance(&mut app, 1);
        }
        assert!(!app.world().resource::<Dialogue>().active);
    }
}

#[test]
fn paused_scenes_preserve_the_arrow_phase_and_discard_elapsed_ticks() {
    for scene in 0..5 {
        let mut app = app(0);
        advance(&mut app, 7);
        match scene {
            0 => {
                app.insert_resource(crate::menu::MenuOpen(true));
            }
            1 => {
                app.insert_resource(crate::shop::ShopOpen(true));
            }
            2 => {
                app.insert_resource(crate::title::TitleActive(true));
            }
            3 => {
                app.insert_resource(crate::gameover::GameOverActive(true));
            }
            _ => {
                let mut transition = crate::transitions::Transition::default();
                transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
                app.insert_resource(transition);
            }
        }
        for _ in 0..25 {
            advance(&mut app, 4);
            assert!(visible(&mut app));
        }
        let world = app.world_mut();
        world.remove_resource::<crate::menu::MenuOpen>();
        world.remove_resource::<crate::shop::ShopOpen>();
        world.remove_resource::<crate::title::TitleActive>();
        world.remove_resource::<crate::gameover::GameOverActive>();
        world.remove_resource::<crate::transitions::Transition>();
        advance(&mut app, 12);
        assert!(visible(&mut app));
        advance(&mut app, 1);
        assert!(!visible(&mut app));
    }
}

#[test]
fn a_clock_wrap_preserves_the_pause_phase() {
    let mut app = app(0);
    app.insert_resource(crate::menu::MenuOpen(true));
    app.world_mut().resource_mut::<GameFrames>().frame = u32::MAX - 1;
    app.update();
    app.world_mut().resource_mut::<crate::menu::MenuOpen>().0 = false;
    advance(&mut app, 1);
    advance(&mut app, 1);
    advance(&mut app, 17);
    assert!(visible(&mut app));
    advance(&mut app, 1);
    assert!(!visible(&mut app));
}

#[test]
fn a_new_message_pause_always_starts_with_its_arrow_visible() {
    for idle in [0, 16, 20, 39, 83] {
        let mut app = app(idle);
        assert!(visible(&mut app), "idle render frames: {idle}");
    }
}

#[test]
fn rendering_without_a_logical_tick_does_not_blink_the_pause_arrow() {
    let mut app = app(0);
    assert!(visible(&mut app));
    for _ in 0..80 {
        app.update();
        assert!(visible(&mut app));
    }
}

#[test]
fn the_visible_and_hidden_halves_last_twenty_logical_ticks_at_every_fps() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = app(0);
        let start = app.world().resource::<GameFrames>().frame;
        for _ in 0..fps * 3 {
            app.world_mut()
                .resource_mut::<GameFrames>()
                .advance(1.0 / f64::from(fps));
            app.update();
            let age = app.world().resource::<GameFrames>().frame - start;
            assert_eq!(visible(&mut app), age % 40 < 20, "{fps} FPS, age {age}");
        }
    }
}
