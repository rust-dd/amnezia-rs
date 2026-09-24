use super::*;
use crate::world::EventSprite;
use bevy::ecs::system::RunSystemOnce;

fn prepared() -> App {
    let mut app = app();
    app.world_mut()
        .run_system_once(
            |mut commands: Commands,
             server: Res<AssetServer>,
             switches: Res<Switches>,
             variables: Res<Variables>,
             party: Res<Party>,
             inventory: Res<Inventory>| {
                crate::world::load_map(
                    &mut commands,
                    &server,
                    &switches,
                    &variables,
                    &party,
                    &inventory,
                    3,
                );
            },
        )
        .unwrap();
    let mut second = map_event(2, 0, vec![]);
    second.pages[0].direction = crate::tiles::DIR_DOWN;
    let mut next = second.pages[0].clone();
    next.direction = crate::tiles::DIR_UP;
    next.condition.flags = 1;
    next.condition.switch_a = 5;
    second.pages.push(next);
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 0, vec![]), second],
    });
    app
}

fn script(app: &mut App, parallel: bool, commands: Vec<EventCommand>) {
    if parallel {
        let mut events = app.world_mut().resource_mut::<MapEvents>();
        events.events[0].pages[0].trigger = 4;
        events.events[0].pages[0].commands = commands;
    } else {
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(1, commands);
    }
}

fn query(variable: i32, property: i32) -> EventCommand {
    cmd(10220, 0, vec![0, variable, variable, 0, 6, 2, property])
}

#[test]
fn an_event_relocation_is_visible_to_the_next_command_in_the_same_burst() {
    for parallel in [false, true] {
        let mut app = prepared();
        script(
            &mut app,
            parallel,
            vec![cmd(10860, 0, vec![2, 0, 2, 3]), query(1, 1), query(2, 2)],
        );
        app.update();
        let variables = app.world().resource::<Variables>();
        assert_eq!((variables.get(1), variables.get(2)), (2, 3));
    }
}

#[test]
fn a_relocation_is_not_replayed_over_a_later_character_update() {
    let mut app = prepared();
    script(
        &mut app,
        true,
        vec![
            cmd(10860, 0, vec![2, 0, 2, 3]),
            cmd(11330, 0, vec![2, 8, 0, 0, 1]),
        ],
    );
    app.update();
    let world = app.world_mut();
    let (npc, queue) = world
        .query::<(&EventSprite, &MoveQueue)>()
        .iter(world)
        .find(|(npc, _)| npc.id == 2)
        .unwrap();
    assert_eq!((npc.tile_x, npc.tile_y), (3, 3));
    assert!(queue.busy());
    assert_eq!(world.resource::<MapEvents>().events[1].x, 3);
}

#[test]
fn page_graphics_refresh_before_the_next_character_query() {
    for parallel in [false, true] {
        let mut app = prepared();
        script(&mut app, parallel, vec![switch_cmd(5, 0, 0), query(1, 3)]);
        app.update();
        assert_eq!(app.world().resource::<Variables>().get(1), 8);
    }
}

#[test]
fn transient_page_changes_refresh_character_state_even_when_the_final_page_is_unchanged() {
    for parallel in [false, true] {
        let mut app = prepared();
        let world = app.world_mut();
        world
            .query::<&mut EventSprite>()
            .iter_mut(world)
            .find(|npc| npc.id == 2)
            .unwrap()
            .dir = crate::tiles::DIR_RIGHT;
        script(
            &mut app,
            parallel,
            vec![switch_cmd(5, 0, 0), switch_cmd(5, 1, 0), query(1, 3)],
        );
        app.update();
        assert_eq!(app.world().resource::<Variables>().get(1), 2);
    }
}

#[test]
fn a_resumed_number_result_refreshes_pages_before_the_next_command() {
    for parallel in [false, true] {
        let mut app = prepared();
        {
            let mut events = app.world_mut().resource_mut::<MapEvents>();
            let condition = &mut events.events[1].pages[1].condition;
            condition.flags = 4;
            condition.variable_id = 7;
            condition.variable_value = 5;
        }
        script(
            &mut app,
            parallel,
            vec![cmd(10150, 0, vec![1, 7]), query(1, 3)],
        );
        app.update();
        assert!(app.world().resource::<InputNumber>().active());
        *app.world_mut().resource_mut::<Dialogue>() = Dialogue::default();
        {
            let mut number = app.world_mut().resource_mut::<InputNumber>();
            number.active = false;
            number.result = Some(5);
        }
        app.update();
        let variables = app.world().resource::<Variables>();
        assert_eq!(variables.get(7), 5);
        assert_eq!(variables.get(1), 8);
    }
}

#[test]
fn command_boundaries_do_not_advance_wait_clocks() {
    let mut app = prepared();
    script(
        &mut app,
        false,
        vec![
            switch_cmd(5, 0, 0),
            query(1, 3),
            cmd(11410, 0, vec![1]),
            switch_cmd(10, 0, 0),
        ],
    );
    app.update();
    assert_eq!(app.world().resource::<RunningEvent>().frame.wait, 0.1);
    app.update();
    let wait = app.world().resource::<RunningEvent>().frame.wait;
    assert!((wait - (0.1 - 1.0 / 60.0)).abs() < 0.000001);
    assert!(!switch_on(&app, 10));
}

#[test]
fn command_budgets_and_resume_points_survive_releasing_world_access() {
    for parallel in [false, true] {
        let mut app = interp_app();
        app.insert_resource(MapEvents {
            events: vec![map_event(1, 0, vec![])],
        });
        let budget = crate::interpreter::frame::MAX_STEPS_PER_FRAME;
        let mut commands = vec![cmd(10220, 0, vec![0, 1, 1, 1, 0, 1]); budget];
        commands.push(switch_cmd(10, 0, 0));
        script(&mut app, parallel, commands);
        app.update();
        assert_eq!(app.world().resource::<Variables>().get(1), budget as i32);
        assert!(!switch_on(&app, 10));
        app.update();
        assert_eq!(app.world().resource::<Variables>().get(1), budget as i32);
        assert!(switch_on(&app, 10));
    }
}
