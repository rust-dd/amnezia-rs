use super::tests::{app_with_event, page};
use super::*;
use crate::tiles::{DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};

#[test]
fn unchanged_page_pose_preserves_direction_independently_of_temporary_facing() {
    let mut original = page("Chara1", 0);
    original.animation_type = 0;
    original.direction = DIR_RIGHT;
    let mut next = original.clone();
    next.condition.flags = 1;
    next.condition.switch_a = 8;
    let mut app = app_with_event(Event {
        id: 1,
        x: 3,
        y: 4,
        name: String::new(),
        pages: vec![original, next],
    });
    let world = app.world_mut();
    let (mut sprite, mut route) = world
        .query::<(&mut EventSprite, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    route.face_toward(&mut *sprite, (3, 3));
    assert_eq!((route.direction(&*sprite), sprite.dir), (DIR_RIGHT, DIR_UP));
    app.world_mut().resource_mut::<Switches>().set(8, true);
    app.update();
    let world = app.world_mut();
    let (mut sprite, route) = world
        .query::<(&mut EventSprite, &RouteStepper)>()
        .single_mut(world)
        .unwrap();
    assert_eq!((route.direction(&*sprite), sprite.dir), (DIR_RIGHT, DIR_UP));
    route.update_facing(&mut *sprite);
    assert_eq!(sprite.dir, DIR_RIGHT);
}

#[test]
fn a_changed_page_pose_resets_both_directions_only_when_stopped() {
    for moving in [false, true] {
        let mut original = page("Chara1", 0);
        original.animation_type = 0;
        original.direction = DIR_RIGHT;
        let mut next = original.clone();
        next.direction = DIR_DOWN;
        next.condition.flags = 1;
        next.condition.switch_a = 8;
        let mut app = app_with_event(Event {
            id: 1,
            x: 3,
            y: 4,
            name: String::new(),
            pages: vec![original, next],
        });
        let world = app.world_mut();
        let (mut sprite, mut route, mut queue) = world
            .query::<(&mut EventSprite, &mut RouteStepper, &mut MoveQueue)>()
            .single_mut(world)
            .unwrap();
        route.set_direction(&mut *sprite, DIR_LEFT);
        route.face_toward(&mut *sprite, (3, 3));
        if moving {
            queue.push_step(crate::world::RouteAction::Step {
                dx: -1,
                dy: 0,
                face: DIR_LEFT,
            });
        }
        app.world_mut().resource_mut::<Switches>().set(8, true);
        app.update();
        let world = app.world_mut();
        let (sprite, route) = world
            .query::<(&EventSprite, &RouteStepper)>()
            .single(world)
            .unwrap();
        assert_eq!(
            (route.direction(sprite), sprite.dir),
            if moving {
                (DIR_LEFT, DIR_UP)
            } else {
                (DIR_DOWN, DIR_DOWN)
            }
        );
    }
}

#[test]
fn losing_the_page_keeps_both_directions_until_a_new_page_is_selected() {
    let mut original = page("Chara1", 0);
    original.animation_type = 0;
    original.direction = DIR_RIGHT;
    original.condition.flags = 1;
    original.condition.switch_a = 8;
    let mut app = app_with_event(Event {
        id: 1,
        x: 3,
        y: 4,
        name: String::new(),
        pages: vec![original],
    });
    app.world_mut().resource_mut::<Switches>().set(8, true);
    app.update();
    let world = app.world_mut();
    let (mut sprite, mut route) = world
        .query::<(&mut EventSprite, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    route.set_direction(&mut *sprite, DIR_LEFT);
    route.face_toward(&mut *sprite, (3, 3));
    app.world_mut().resource_mut::<Switches>().set(8, false);
    app.update();
    let world = app.world_mut();
    let (sprite, route) = world
        .query::<(&EventSprite, &RouteStepper)>()
        .single(world)
        .unwrap();
    assert_eq!((route.direction(sprite), sprite.dir), (DIR_LEFT, DIR_UP));
    app.world_mut().resource_mut::<Switches>().set(8, true);
    app.update();
    let world = app.world_mut();
    let (sprite, route) = world
        .query::<(&EventSprite, &RouteStepper)>()
        .single(world)
        .unwrap();
    assert_eq!(
        (route.direction(sprite), sprite.dir),
        (DIR_RIGHT, DIR_RIGHT)
    );
}
