use super::equip_switching::{prepared, screen};
use super::*;
use crate::menu::equip::{Scene, Switch};
use crate::timing::GameFrames;
use crate::transitions::{Transition, TransitionPlugin};

#[derive(Resource, Default)]
struct Sounds(Vec<AudioRequest>);

fn heard(app: &mut App) -> Vec<AudioRequest> {
    std::mem::take(&mut app.world_mut().resource_mut::<Sounds>().0)
}

fn record_audio(mut messages: ResMut<Messages<AudioRequest>>, mut sounds: ResMut<Sounds>) {
    sounds.0.extend(messages.drain());
}

fn fixture(slot: usize) -> App {
    let mut app = prepared(slot);
    app.add_plugins(TransitionPlugin);
    equip::register_switching(&mut app);
    app.init_resource::<Sounds>()
        .add_systems(PostUpdate, record_audio);
    app.update();
    app
}

fn tick(app: &mut App, keys: &[KeyCode]) {
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.reset_all();
    for key in keys {
        input.press(*key);
    }
    app.update();
}

#[test]
fn switching_erases_the_old_actor_before_showing_the_new_scene_for_six_frames_each() {
    for slot in 0..5 {
        let mut app = fixture(slot);
        tick(&mut app, &[KeyCode::ArrowRight]);
        let old_phase = app.world().resource::<Scene>().slot_frame;
        for age in 0_u32..7 {
            assert_eq!(
                app.world().resource::<MenuState>().screen,
                screen(0, slot, None)
            );
            let scene = app.world().resource::<Scene>();
            assert_eq!(scene.member, Some(0));
            assert_eq!(scene.slot_frame, old_phase);
            assert!(app.world().resource::<Switch>().active());
            assert_eq!(
                app.world().resource::<Transition>().age(),
                age.saturating_sub(1)
            );
            tick(&mut app, &[]);
        }
        for age in 0_u32..7 {
            assert_eq!(
                app.world().resource::<MenuState>().screen,
                screen(1, slot, None)
            );
            let scene = app.world().resource::<Scene>();
            assert_eq!((scene.member, scene.current), (Some(1), [45, 20, 17, 14]));
            assert_eq!(scene.slot_frame, 0);
            assert!(
                scene
                    .lists
                    .iter()
                    .all(|list| list.arrow_frame == 0 && list.offset == 0)
            );
            assert_eq!(scene.help_id, if slot == 0 { 11 } else { 0 });
            assert_eq!(
                app.world().resource::<Transition>().age(),
                age.saturating_sub(1)
            );
            tick(&mut app, &[]);
        }
        assert!(!app.world().resource::<Switch>().active());
        assert!(!app.world().resource::<Transition>().busy());
        assert!(!app.world().resource::<Transition>().erased());
        assert_eq!(app.world().resource::<Scene>().slot_frame, 1);
        assert_eq!(
            heard(&mut app),
            [AudioRequest::se("CURSOR", 100, 100).unwrap()]
        );
    }
}

#[test]
fn both_halves_block_equipping_cancelling_saving_and_navigation_without_replaying_keys() {
    let mut app = fixture(0);
    tick(&mut app, &[KeyCode::ArrowRight]);
    for age in 1..14 {
        tick(
            &mut app,
            &[
                KeyCode::Enter,
                KeyCode::Space,
                KeyCode::Escape,
                KeyCode::KeyS,
                KeyCode::ArrowRight,
                KeyCode::ArrowLeft,
                KeyCode::ArrowDown,
                KeyCode::PageDown,
            ],
        );
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(usize::from(age >= 7), 0, None)
        );
        assert!(app.world().resource::<MenuOpen>().0);
        assert!(!app.world().resource::<SaveFiles>().active());
        assert!(!app.world().resource::<SaveRequest>().0);
    }
    tick(&mut app, &[]);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(1, 0, None)
    );
    assert!(app.world().resource::<Equipment>().entries().is_empty());
    for id in 10..40 {
        assert_eq!(app.world().resource::<Inventory>().count(id), 1);
    }
    assert_eq!(
        heard(&mut app),
        [AudioRequest::se("CURSOR", 100, 100).unwrap()]
    );
    tick(&mut app, &[KeyCode::Enter]);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(1, 0, Some(0))
    );
}

#[test]
fn zero_tick_render_updates_do_not_finish_fades_or_animate_either_scene() {
    let mut app = fixture(2);
    tick(&mut app, &[KeyCode::ArrowRight]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    for _ in 0..40 {
        app.update();
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(0, 2, None)
        );
        assert_eq!(app.world().resource::<Transition>().age(), 0);
        assert_eq!(app.world().resource::<Scene>().slot_frame, 1);
    }
    app.world_mut().resource_mut::<GameFrames>().frame += 7;
    app.update();
    for _ in 0..40 {
        app.update();
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(1, 2, None)
        );
        assert_eq!(app.world().resource::<Transition>().age(), 0);
        assert_eq!(app.world().resource::<Scene>().slot_frame, 0);
    }
}

#[test]
fn held_switch_key_is_consumed_once_across_both_transitions_at_all_render_rates() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = fixture(3);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        for rendered in 1..=fps {
            app.world_mut().resource_mut::<GameFrames>().frame = rendered * 60 / fps;
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
        }
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(1, 3, None),
            "{fps} FPS"
        );
        assert!(!app.world().resource::<Switch>().active());
        assert_eq!(
            heard(&mut app),
            [AudioRequest::se("CURSOR", 100, 100).unwrap()]
        );
        tick(&mut app, &[KeyCode::ArrowRight]);
        assert!(app.world().resource::<Switch>().active());
        for _ in 0..14 {
            tick(&mut app, &[]);
        }
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(2, 3, None)
        );
    }
}

#[test]
fn slot_movement_is_visible_in_the_outgoing_snapshot_and_preserved_after_the_fade() {
    let mut app = fixture(2);
    tick(&mut app, &[KeyCode::ArrowDown, KeyCode::ArrowRight]);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(0, 3, None)
    );
    for _ in 0..14 {
        tick(&mut app, &[]);
    }
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(1, 3, None)
    );
    assert_eq!(
        heard(&mut app),
        vec![AudioRequest::se("CURSOR", 100, 100).unwrap(); 2]
    );
}

#[test]
fn picker_and_single_member_navigation_do_not_start_actor_transitions() {
    let mut app = fixture(0);
    tick(&mut app, &[KeyCode::Enter]);
    tick(&mut app, &[KeyCode::ArrowRight]);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(0, 0, Some(1))
    );
    assert!(!app.world().resource::<Switch>().active());
    assert!(!app.world().resource::<Transition>().busy());
    tick(&mut app, &[KeyCode::Escape]);
    app.world_mut().resource_mut::<Party>().restore(vec![1]);
    heard(&mut app);
    tick(&mut app, &[KeyCode::ArrowRight]);
    assert!(!app.world().resource::<Switch>().active());
    assert!(!app.world().resource::<Transition>().busy());
    assert!(heard(&mut app).is_empty());
}

#[test]
fn clearing_the_session_cancels_pending_actor_replacement_and_both_fade_halves() {
    for ticks in [0, 7] {
        let mut app = fixture(1);
        tick(&mut app, &[KeyCode::ArrowRight]);
        for _ in 0..ticks {
            tick(&mut app, &[]);
        }
        crate::session::clear_transient(app.world_mut());
        assert!(!app.world().resource::<Switch>().active());
        assert!(!app.world().resource::<Transition>().busy());
        app.world_mut().resource_mut::<MenuState>().screen = screen(0, 0, None);
        for _ in 0..20 {
            tick(&mut app, &[]);
        }
        assert!(!app.world().resource::<MenuOpen>().0);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(0, 0, None)
        );
    }
}
