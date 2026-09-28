use super::*;
use crate::menu::SceneFlow;
use crate::timing::TimingPlugin;
use crate::transitions::{Transition, TransitionPlugin};

mod arbitration;
mod lifecycle;

fn fixture() -> App {
    let mut app = app();
    crate::menu::register_map_input(&mut app);
    app.add_plugins((TimingPlugin, TransitionPlugin));
    app.update();
    app
}

fn tick(app: &mut App, keys: &[KeyCode]) {
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.reset_all();
    for key in keys {
        input.press(*key);
    }
    app.update();
}

fn calling(app: &App) -> bool {
    app.world().resource::<SceneFlow>().active()
}

fn busy(app: &mut App) -> bool {
    let world = app.world_mut();
    world
        .query_filtered::<&MoveQueue, With<Player>>()
        .single(world)
        .unwrap()
        .busy()
}

fn floor_event(app: &mut App, trigger: u32, x: u32) {
    let mut event = map_event(1, trigger, vec![switch_cmd(10, 0, 0)]);
    event.x = x;
    event.y = 5;
    app.insert_resource(MapEvents {
        events: vec![event],
    });
}

#[test]
fn fresh_cancel_is_latched_until_the_next_stopped_player_update() {
    let mut app = fixture();
    tick(&mut app, &[KeyCode::Escape]);
    assert!(!calling(&app));
    assert!(!app.world().resource::<MenuOpen>().0);
    tick(&mut app, &[]);
    assert!(calling(&app));
    assert_eq!(app.world().resource::<Transition>().age(), 0);
    assert!(!app.world().resource::<MenuOpen>().0);
    for _ in 0..7 {
        tick(&mut app, &[]);
    }
    assert!(app.world().resource::<MenuOpen>().0);
    assert_eq!(app.world().resource::<Transition>().age(), 0);
}

#[test]
fn movement_precedes_cancel_and_the_latch_waits_past_the_last_tween_tick() {
    let mut app = fixture();
    tick(&mut app, &[KeyCode::ArrowRight, KeyCode::Escape]);
    assert_eq!(hero_x(&mut app), 6);
    assert!(busy(&mut app));
    assert!(!calling(&app));
    for _ in 0..20 {
        if !busy(&mut app) {
            break;
        }
        tick(&mut app, &[]);
        assert!(!calling(&app));
    }
    assert!(!busy(&mut app));
    tick(&mut app, &[KeyCode::ArrowRight]);
    assert!(calling(&app));
    assert_eq!(hero_x(&mut app), 6);
}

#[test]
fn a_simultaneous_short_decision_event_discards_the_new_menu_latch() {
    let mut app = fixture();
    crate::dialogue::testing::register_actions(&mut app);
    floor_event(&mut app, 0, 5);
    tick(&mut app, &[KeyCode::Enter, KeyCode::Escape]);
    assert!(switch_on(&app, 10));
    assert!(!app.world().resource::<RunningEvent>().active());
    assert!(!calling(&app));
    tick(&mut app, &[]);
    assert!(!calling(&app));
}

#[test]
fn a_short_arrival_event_discards_cancel_captured_on_the_final_step_tick() {
    let mut app = fixture();
    floor_event(&mut app, 1, 6);
    let world = app.world_mut();
    queue_hero_step(world, crate::tiles::DIR_RIGHT, 6);
    tick(&mut app, &[]);
    tick(&mut app, &[KeyCode::Escape]);
    assert!(switch_on(&app, 10));
    assert!(!calling(&app));
    tick(&mut app, &[]);
    assert!(!calling(&app));
}

#[test]
fn a_foreground_start_cancels_an_older_latch_even_if_it_finishes_immediately() {
    let mut app = fixture();
    tick(&mut app, &[KeyCode::Escape]);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, vec![switch_cmd(10, 0, 0)]);
    tick(&mut app, &[]);
    assert!(switch_on(&app, 10));
    assert!(!calling(&app));
    tick(&mut app, &[]);
    assert!(!calling(&app));
}

#[test]
fn a_forced_route_holds_the_latch_until_it_releases_player_control() {
    let mut app = fixture();
    let world = app.world_mut();
    *world
        .query::<&mut RouteStepper>()
        .single_mut(world)
        .unwrap() = RouteStepper::from_move_event(&[10001, 3, 0, 0, 23]);
    tick(&mut app, &[KeyCode::Escape]);
    for _ in 0..4 {
        assert!(!calling(&app));
        tick(&mut app, &[]);
    }
    let world = app.world_mut();
    *world
        .query::<&mut RouteStepper>()
        .single_mut(world)
        .unwrap() = default();
    tick(&mut app, &[]);
    assert!(calling(&app));
}

#[test]
fn menu_access_is_checked_at_capture_not_when_consuming_a_valid_latch() {
    let mut app = fixture();
    tick(&mut app, &[KeyCode::Escape]);
    app.world_mut().resource_mut::<MenuAccess>().0 = false;
    tick(&mut app, &[]);
    assert!(calling(&app));
    let mut app = fixture();
    app.world_mut().resource_mut::<MenuAccess>().0 = false;
    tick(&mut app, &[KeyCode::Escape]);
    app.world_mut().resource_mut::<MenuAccess>().0 = true;
    tick(&mut app, &[]);
    assert!(!calling(&app));
}

#[test]
fn a_latched_menu_precedes_new_floor_contacts_but_allows_one_queued_command() {
    let mut app = fixture();
    tick(&mut app, &[KeyCode::Escape]);
    floor_event(&mut app, 2, 5);
    tick(&mut app, &[]);
    assert!(calling(&app));
    assert!(!switch_on(&app, 10));

    let mut app = fixture();
    tick(&mut app, &[KeyCode::Escape]);
    floor_event(&mut app, 0, 5);
    app.world_mut().resource_mut::<MapEvents>().events[0].pages[0]
        .commands
        .push(switch_cmd(11, 0, 0));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .queue_event(0, 1, 0, false);
    tick(&mut app, &[]);
    assert!(calling(&app));
    assert!(switch_on(&app, 10));
    assert!(!switch_on(&app, 11));
}
