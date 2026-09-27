use super::*;
use crate::world::{AutoMove, EventSprite};

mod lifecycle;

fn x(app: &App, entity: Entity) -> i32 {
    app.world().get::<EventSprite>(entity).unwrap().tile_x
}

fn hero_at(app: &mut App, x: i32, y: i32) {
    let world = app.world_mut();
    let mut hero = world.query::<&mut Player>().single_mut(world).unwrap();
    hero.tile_x = x;
    hero.tile_y = y;
}

#[test]
fn a_blocked_forced_move_retries_on_the_next_character_update_at_every_frequency() {
    for frequency in 1..=8 {
        let mut app = app();
        hero_at(&mut app, 2, 1);
        let entity = npcs::npc(&mut app, 1, 1, vec![], &[1, frequency, 0, 0, 1]);
        app.update();
        assert_eq!(x(&app, entity), 1);
        hero_at(&mut app, 2, 2);
        app.update();
        assert_eq!(x(&app, entity), 2, "frequency {frequency}");
    }
}

#[test]
fn a_blocked_forced_jump_does_not_install_a_new_frequency_delay() {
    let mut app = app();
    hero_at(&mut app, 2, 1);
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[1, 1, 0, 0, 24, 1, 25]);
    app.update();
    assert_eq!(x(&app, entity), 1);
    hero_at(&mut app, 2, 2);
    app.update();
    assert_eq!(x(&app, entity), 2);
    assert!(app.world().get::<MoveQueue>(entity).unwrap().jumping());
}

#[test]
fn frequency_eight_turns_can_reach_the_next_move_in_the_same_update() {
    let mut app = app();
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[1, 8, 0, 0, 12, 13, 1]);
    app.update();
    assert_eq!(x(&app, entity), 2);
    assert_eq!(app.world().get::<EventSprite>(entity).unwrap().dir, 1);
}

#[test]
fn a_final_frequency_eight_turn_finishes_the_forced_route_immediately() {
    let mut app = app();
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[1, 8, 0, 0, 12]);
    app.update();
    let route = app.world().get::<RouteStepper>(entity).unwrap();
    assert!(!route.forced());
    assert!(!route.pending());
    assert_eq!(app.world().get::<EventSprite>(entity).unwrap().dir, 0);
}

#[test]
fn instant_forced_completion_preserves_the_stop_count_for_slow_autonomy() {
    let mut app = app();
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[]);
    let mut route = RouteStepper::from_page(&amnezia_data::MoveRouteDef::default(), 4, 3);
    route.force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 32, 7]));
    app.world_mut()
        .entity_mut(entity)
        .insert((route, AutoMove::new(3, 3, 4, 1)));
    app.update();
    assert!(switch_on(&app, 7));
    assert_eq!(x(&app, entity), 2);
}

#[test]
fn a_page_change_keeps_accumulated_stops_instead_of_starting_a_fresh_delay() {
    let mut app = app();
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[]);
    let mut page = map_event(1, 0, vec![]).pages.remove(0);
    page.move_type = 0;
    page.move_frequency = 3;
    page.move_route = default();
    app.world_mut()
        .entity_mut(entity)
        .insert(RouteStepper::from_event_page(Some(&page)));
    for _ in 0..20 {
        app.update();
    }
    page.move_type = 6;
    page.move_frequency = 6;
    page.move_route.commands = vec![amnezia_data::MoveCommandDef {
        code: 1,
        ..default()
    }];
    app.world_mut()
        .get_mut::<RouteStepper>(entity)
        .unwrap()
        .refresh_page(Some(&page));
    app.update();
    assert_eq!(x(&app, entity), 2);
}

#[test]
fn a_custom_page_counts_stopped_updates_before_its_first_command() {
    let mut app = app();
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[]);
    let mut page = map_event(1, 0, vec![]).pages.remove(0);
    page.move_type = 6;
    page.move_frequency = 7;
    page.move_route.commands = vec![amnezia_data::MoveCommandDef {
        code: 1,
        ..default()
    }];
    app.world_mut()
        .entity_mut(entity)
        .insert(RouteStepper::from_event_page(Some(&page)));
    for tick in 1..=2 {
        app.update();
        assert_eq!(x(&app, entity), 1, "tick {tick}");
    }
    app.update();
    assert_eq!(x(&app, entity), 2);
}

#[test]
fn a_skipped_move_installs_the_step_threshold_without_resetting_the_count() {
    let mut app = app();
    hero_at(&mut app, 2, 1);
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[1, 7, 0, 1, 12, 1, 32, 7]);
    for tick in 1..=4 {
        app.update();
        assert!(!switch_on(&app, 7), "tick {tick}");
        assert_eq!(x(&app, entity), 1);
    }
    app.update();
    assert!(switch_on(&app, 7));
    assert_eq!(app.world().get::<EventSprite>(entity).unwrap().dir, 0);
}
