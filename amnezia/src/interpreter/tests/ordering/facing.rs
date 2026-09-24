use super::*;
use crate::tiles::{DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use crate::world::{Character, EventSprite};

mod vehicles;

fn setup(mode: u32) -> (App, Entity) {
    let mut app = app();
    crate::dialogue::testing::register_actions(&mut app);
    let entity = super::npcs::npc(&mut app, 1, 5, vec![cmd(11410, 0, vec![100])], &[]);
    let route = {
        let mut events = app.world_mut().resource_mut::<MapEvents>();
        let event = &mut events.events[0];
        event.y = 6;
        event.pages[0].trigger = 0;
        event.pages[0].direction = DIR_RIGHT;
        event.pages[0].animation_type = mode;
        RouteStepper::from_event_page(Some(&event.pages[0]))
    };
    app.world_mut().entity_mut(entity).insert(route);
    app.world_mut()
        .get_mut::<EventSprite>(entity)
        .unwrap()
        .tile_y = 6;
    (app, entity)
}

fn talk(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
}

fn directions(app: &App, entity: Entity) -> (u32, u32) {
    let sprite = app.world().get::<EventSprite>(entity).unwrap();
    let route = app.world().get::<RouteStepper>(entity).unwrap();
    (route.direction(sprite), sprite.dir)
}

#[test]
fn talking_faces_the_hero_without_changing_direction_and_finishing_restores_it() {
    let (mut app, entity) = setup(0);
    talk(&mut app);
    assert_eq!(directions(&app, entity), (DIR_RIGHT, DIR_UP));
    app.world_mut().resource_mut::<RunningEvent>().frame.wait = 0.0;
    app.update();
    assert_eq!(directions(&app, entity), (DIR_RIGHT, DIR_RIGHT));
}

#[test]
fn talking_respects_page_facing_locks_and_spinning_animation() {
    for mode in 0..=6 {
        let (mut app, entity) = setup(mode);
        talk(&mut app);
        assert_eq!(
            directions(&app, entity),
            (
                DIR_RIGHT,
                if (2..=5).contains(&mode) {
                    DIR_RIGHT
                } else {
                    DIR_UP
                }
            ),
            "animation type {mode}"
        );
    }
}

#[test]
fn temporary_facing_uses_the_shortest_wrapped_distance() {
    let (mut app, entity) = setup(0);
    app.world_mut().resource_mut::<MapData>().scroll_type = 2;
    let world = app.world_mut();
    let mut hero = world.query::<&mut Player>().single_mut(world).unwrap();
    hero.tile_x = 0;
    hero.dir = DIR_LEFT;
    let mut sprite = world.get_mut::<EventSprite>(entity).unwrap();
    sprite.tile_x = 9;
    sprite.tile_y = 5;
    sprite.dir = DIR_DOWN;
    let mut route = world.get_mut::<RouteStepper>(entity).unwrap();
    *route = RouteStepper::default();
    let mut events = world.resource_mut::<MapEvents>();
    events.events[0].x = 9;
    events.events[0].y = 5;
    talk(&mut app);
    assert_eq!(directions(&app, entity), (DIR_DOWN, DIR_RIGHT));
}

#[test]
fn finishing_a_nested_call_does_not_restore_the_base_events_facing_early() {
    let (mut app, entity) = setup(0);
    let mut events = app.world_mut().resource_mut::<MapEvents>();
    events.events[0].pages[0]
        .commands
        .insert(0, cmd(12330, 0, vec![1, 2, 1]));
    events
        .events
        .push(map_event(2, 0, vec![switch_cmd(10, 0, 0)]));
    talk(&mut app);
    assert!(switch_on(&app, 10));
    assert_eq!(directions(&app, entity), (DIR_RIGHT, DIR_UP));
    app.world_mut().resource_mut::<RunningEvent>().frame.wait = 0.0;
    app.update();
    assert_eq!(directions(&app, entity), (DIR_RIGHT, DIR_RIGHT));
}

#[test]
fn decision_targets_the_heros_logical_direction_when_facing_is_locked() {
    let (mut app, _) = setup(0);
    let world = app.world_mut();
    let (mut hero, mut route, mut queue) = world
        .query::<(&mut Player, &mut RouteStepper, &mut MoveQueue)>()
        .single_mut(world)
        .unwrap();
    hero.dir = DIR_LEFT;
    route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 26]));
    crate::world::drive_route(
        &mut *hero,
        &mut queue,
        &mut route,
        (0, 0),
        1.0 / 60.0,
        |_, _, _, _, _| true,
    );
    route.set_direction(&mut *hero, DIR_DOWN);
    assert_eq!(hero.dir(), DIR_LEFT);
    talk(&mut app);
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(1));
}
