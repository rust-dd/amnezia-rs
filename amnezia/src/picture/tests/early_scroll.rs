use super::*;
use crate::player::{CameraPan, Player};
use crate::world::{Character, MapEvents, RouteStepper, test_support};
use amnezia_data::{Event, EventCommand};
use bevy::ecs::system::RunSystemOnce;

mod logical;

fn event(id: u32, tile: (u32, u32), direction: Option<u32>) -> Event {
    let mut page = test_support::page(
        direction
            .into_iter()
            .map(|direction| test_support::command(direction, 0))
            .collect(),
    );
    page.trigger = 4;
    page.commands = vec![
        EventCommand {
            code: 11110,
            indent: 0,
            string: "Cross".into(),
            params: vec![
                id as i32, 0, 160, 120, 1, 100, 0, 0, 100, 100, 100, 100, 0, 0,
            ],
        },
        EventCommand {
            code: 11410,
            indent: 0,
            string: String::new(),
            params: vec![100],
        },
    ];
    let mut event = test_support::event(id, tile.0, vec![page]);
    event.y = tile.1;
    event
}

fn install(app: &mut App, events: Vec<Event>) {
    app.insert_resource(MapEvents { events });
    app.world_mut()
        .run_system_once(test_support::spawn)
        .unwrap();
}

#[test]
fn a_picture_shown_before_an_early_hero_walk_receives_that_walks_scroll() {
    let (mut app, hero) = super::scroll::fixture();
    install(&mut app, vec![event(1, (19, 15), Some(1))]);
    app.world_mut()
        .get_mut::<RouteStepper>(hero)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 1]));
    app.update();
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (21, 15));
    assert_eq!(
        super::scroll::relative_positions(app.world_mut()),
        [Vec2::new(-2.0, 0.0)]
    );
}

#[test]
fn pictures_between_repeated_hero_visits_receive_only_subsequent_pan_steps() {
    let (mut app, _) = super::scroll::fixture();
    install(
        &mut app,
        vec![
            event(1, (19, 15), Some(1)),
            event(2, (20, 14), Some(2)),
            event(3, (30, 15), None),
        ],
    );
    app.world_mut()
        .resource_mut::<CameraPan>()
        .command(&[2, 1, 1, 4, 0]);
    app.update();
    assert_eq!(app.world().resource::<CameraPan>().offset, Vec2::X * 6.0);
    assert_eq!(
        super::scroll::relative_positions(app.world_mut()),
        [
            Vec2::new(-6.0, 0.0),
            Vec2::new(-4.0, 0.0),
            Vec2::new(-2.0, 0.0),
        ]
    );
}

#[test]
fn pictures_from_nested_destination_scripts_precede_the_calling_heroes_scroll() {
    let (mut app, hero) = super::scroll::fixture();
    let mut target = event(2, (21, 15), None);
    target.pages[0].layer = 2;
    install(&mut app, vec![event(1, (19, 15), Some(1)), target]);
    app.world_mut()
        .get_mut::<RouteStepper>(hero)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 1]));
    app.update();
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (21, 15));
    assert_eq!(
        super::scroll::relative_positions(app.world_mut()),
        [Vec2::new(-2.0, 0.0); 2]
    );
}

#[test]
fn repeated_hero_visits_do_not_repeat_picture_commands_or_tween_ticks() {
    let (mut app, _) = super::scroll::fixture();
    let mut first = event(1, (19, 15), Some(1));
    first.pages[0].commands.insert(
        1,
        EventCommand {
            code: 11120,
            indent: 0,
            string: String::new(),
            params: vec![1, 0, 160, 120, 1, 100, 100, 0, 100, 100, 100, 100, 0, 0, 10],
        },
    );
    install(&mut app, vec![first, event(2, (20, 14), Some(2))]);
    app.world_mut()
        .resource_mut::<CameraPan>()
        .command(&[2, 1, 1, 4, 0]);
    for elapsed in 1..=10 {
        app.update();
        let world = app.world_mut();
        let picture = world
            .query::<&Picture>()
            .iter(world)
            .find(|picture| picture.id == 1)
            .unwrap();
        let tween = picture.tween.unwrap();
        assert_eq!(tween.elapsed, elapsed);
        assert_eq!(tween.frames, 60);
    }
}
