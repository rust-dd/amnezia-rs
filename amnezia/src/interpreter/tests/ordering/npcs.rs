use super::*;
use crate::world::{AutoMove, EventSprite};

pub(super) fn npc(
    app: &mut App,
    id: u32,
    x: i32,
    commands: Vec<EventCommand>,
    route: &[i32],
) -> Entity {
    let mut event = map_event(id, 4, commands);
    event.x = x as u32;
    event.y = 1;
    event.pages[0].layer = 1;
    app.world_mut()
        .resource_mut::<MapEvents>()
        .events
        .push(event);
    app.world_mut()
        .spawn((
            EventSprite {
                id,
                tile_x: x,
                tile_y: 1,
                dir: 1,
                frame: 1,
                charset: "Chara1".into(),
                index: 0,
                layer: 1,
            },
            MoveQueue::default(),
            RouteStepper::from_move_event(route),
            AutoMove::new(0, 8, 4, id),
            Sprite::default(),
            Transform::default(),
        ))
        .id()
}

fn query(variable: i32, target: i32) -> EventCommand {
    cmd(10220, 0, vec![0, variable, variable, 0, 6, target, 1])
}

fn x(app: &App, entity: Entity) -> i32 {
    app.world().get::<EventSprite>(entity).unwrap().tile_x
}

#[test]
fn each_parallel_page_observes_the_previous_npcs_completed_movement_update() {
    let mut app = app();
    npc(&mut app, 2, 6, vec![query(2, 1)], &[]);
    let first = npc(&mut app, 1, 1, vec![query(1, 1)], &[1, 8, 0, 0, 1]);
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(1), 1);
    assert_eq!(app.world().resource::<Variables>().get(2), 2);
    assert_eq!(x(&app, first), 2);
    let destination = app.world().resource::<MapData>().tile_center(2, 1).0;
    assert!(app.world().get::<Transform>(first).unwrap().translation.x < destination);
}

#[test]
fn a_route_switch_can_enable_a_later_parallel_page_in_the_same_update() {
    let mut app = app();
    npc(&mut app, 1, 1, vec![], &[1, 8, 0, 0, 32, 5]);
    npc(&mut app, 2, 6, vec![switch_cmd(10, 0, 0)], &[]);
    let mut events = app.world_mut().resource_mut::<MapEvents>();
    events.events[1].pages[0].condition.flags = 1;
    events.events[1].pages[0].condition.switch_a = 5;
    app.update();
    assert!(switch_on(&app, 5));
    assert!(switch_on(&app, 10));
}

#[test]
fn a_later_parallel_route_cannot_retroactively_move_an_earlier_npc() {
    let mut app = app();
    let first = npc(&mut app, 1, 1, vec![], &[]);
    npc(
        &mut app,
        2,
        6,
        vec![cmd(11330, 0, vec![1, 8, 0, 0, 1])],
        &[],
    );
    app.update();
    assert_eq!(x(&app, first), 1);
    app.update();
    assert_eq!(x(&app, first), 2);
}

#[test]
fn an_earlier_parallel_route_can_move_a_later_npc_immediately() {
    let mut app = app();
    npc(
        &mut app,
        1,
        1,
        vec![cmd(11330, 0, vec![2, 8, 0, 0, 1])],
        &[],
    );
    let second = npc(&mut app, 2, 6, vec![query(1, 2)], &[]);
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(1), 6);
    assert_eq!(x(&app, second), 7);
}

#[test]
fn autonomous_movement_also_precedes_the_next_parallel_page() {
    let mut app = app();
    let first = npc(&mut app, 1, 1, vec![], &[]);
    app.world_mut()
        .entity_mut(first)
        .insert(AutoMove::new(3, 8, 4, 1));
    npc(&mut app, 2, 6, vec![query(1, 1)], &[]);
    app.update();
    assert_eq!(x(&app, first), 2);
    assert_eq!(app.world().resource::<Variables>().get(1), 2);
}

#[test]
fn finishing_an_instant_forced_route_at_frequency_eight_resumes_autonomy_immediately() {
    let mut app = app();
    let first = npc(&mut app, 1, 1, vec![], &[]);
    let mut route = RouteStepper::from_page(&amnezia_data::MoveRouteDef::default(), 4, 8);
    route.force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 32, 5]));
    app.world_mut()
        .entity_mut(first)
        .insert((route, AutoMove::new(3, 8, 4, 1)));
    npc(&mut app, 2, 6, vec![query(1, 1)], &[]);
    app.update();
    assert!(switch_on(&app, 5));
    assert_eq!(x(&app, first), 2);
    assert_eq!(app.world().resource::<Variables>().get(1), 2);
}

#[test]
fn event_id_order_decides_contested_movement_independently_of_spawn_order() {
    let mut app = app();
    let second = npc(&mut app, 2, 4, vec![], &[2, 8, 0, 0, 3]);
    let first = npc(&mut app, 1, 2, vec![], &[1, 8, 0, 0, 1]);
    for event in &mut app.world_mut().resource_mut::<MapEvents>().events {
        event.pages[0].trigger = 0;
    }
    app.update();
    assert_eq!(x(&app, first), 3);
    assert_eq!(x(&app, second), 4);
}

#[test]
fn a_route_page_change_refreshes_later_character_properties_before_its_interpreter() {
    use bevy::ecs::system::RunSystemOnce;
    let mut app = app();
    app.world_mut()
        .run_system_once(
            |mut commands: Commands,
             server: Res<AssetServer>,
             switches: Res<Switches>,
             variables: Res<Variables>,
             party: Res<Party>,
             inventory: Res<Inventory>| {
                let (data, events) = crate::world::load_map(
                    &mut commands,
                    &server,
                    &switches,
                    &variables,
                    &party,
                    &inventory,
                    3,
                );
                commands.insert_resource(data);
                commands.insert_resource(events);
            },
        )
        .unwrap();
    let first = app
        .world_mut()
        .query::<(Entity, &EventSprite)>()
        .iter(app.world())
        .find(|(_, event)| event.id == 1)
        .unwrap()
        .0;
    app.world_mut()
        .entity_mut(first)
        .insert(RouteStepper::from_move_event(&[1, 8, 0, 0, 32, 5]));
    let mut second = map_event(2, 0, vec![]);
    let mut next = map_event(2, 4, vec![cmd(10220, 0, vec![0, 1, 1, 0, 6, 2, 3])])
        .pages
        .remove(0);
    next.condition.flags = 1;
    next.condition.switch_a = 5;
    next.direction = crate::tiles::DIR_UP;
    next.graphic_name = "Chara1".into();
    second.pages.push(next);
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 0, vec![]), second],
    });
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(1), 8);
}
