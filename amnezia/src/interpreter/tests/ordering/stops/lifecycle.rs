use super::*;

fn count(app: &App, entity: Entity) -> u32 {
    app.world()
        .get::<RouteStepper>(entity)
        .unwrap()
        .stop_count()
}

#[test]
fn a_zero_stop_count_ticks_once_under_foreground_pause_but_other_counts_hold() {
    for (initial, expected) in [(0, 1), (7, 7)] {
        let mut app = app();
        let entity = npcs::npc(&mut app, 1, 1, vec![], &[]);
        app.world_mut()
            .get_mut::<RouteStepper>(entity)
            .unwrap()
            .set_stop_count(initial);
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(0, vec![cmd(11410, 0, vec![100])]);
        for _ in 0..5 {
            app.update();
            assert_eq!(count(&app, entity), expected);
        }
    }
}

#[test]
fn continue_events_does_not_advance_the_speaking_characters_clock() {
    let mut app = app();
    let speaking = npcs::npc(&mut app, 1, 1, vec![], &[]);
    let other = npcs::npc(&mut app, 2, 3, vec![], &[]);
    for entity in [speaking, other] {
        app.world_mut()
            .get_mut::<RouteStepper>(entity)
            .unwrap()
            .set_stop_count(5);
    }
    app.world_mut()
        .resource_mut::<crate::dialogue::MessageOptions>()
        .continue_events = true;
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, vec![cmd(11410, 0, vec![100])]);
    for tick in 1..=5 {
        app.update();
        assert_eq!(count(&app, speaking), 5);
        assert_eq!(count(&app, other), 5 + tick);
    }
}

#[test]
fn forced_waits_advance_despite_foreground_pause_and_menu_holds_the_whole_clock() {
    let mut app = app();
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[1, 8, 0, 0, 23, 1]);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, vec![cmd(11410, 0, vec![100])]);
    for tick in 1..=4 {
        app.update();
        assert_eq!(count(&app, entity), tick);
    }
    app.world_mut().resource_mut::<MenuOpen>().0 = true;
    for _ in 0..4 {
        app.update();
        assert_eq!(count(&app, entity), 4);
    }
    app.world_mut().resource_mut::<MenuOpen>().0 = false;
    app.update();
    assert_eq!(count(&app, entity), 5);
}

#[test]
fn movement_keeps_zero_through_landing_and_counts_stops_on_the_following_update() {
    let mut app = app();
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[1, 1, 0, 0, 1]);
    app.world_mut()
        .get_mut::<RouteStepper>(entity)
        .unwrap()
        .set_speed(6);
    app.update();
    assert_eq!(count(&app, entity), 0);
    assert!(app.world().get::<MoveQueue>(entity).unwrap().busy());
    app.update();
    assert!(!app.world().get::<MoveQueue>(entity).unwrap().busy());
    assert!(!app.world().get::<RouteStepper>(entity).unwrap().forced());
    assert_eq!(count(&app, entity), 0);
    app.update();
    assert_eq!(count(&app, entity), 1);
}

#[test]
fn a_page_less_character_does_not_advance_its_stop_clock() {
    let mut app = app();
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[]);
    {
        let mut events = app.world_mut().resource_mut::<MapEvents>();
        events.events[0].pages[0].condition.flags = 1;
        events.events[0].pages[0].condition.switch_a = 99;
    }
    {
        let mut route = app.world_mut().get_mut::<RouteStepper>(entity).unwrap();
        route.set_stop_count(7);
        route.refresh_page(None);
    }
    for _ in 0..4 {
        app.update();
        assert_eq!(count(&app, entity), 7);
    }
}

#[test]
fn collision_resets_the_count_even_when_the_event_has_no_commands() {
    for empty in [false, true] {
        for floor in [false, true] {
            let mut app = app();
            hero_at(&mut app, if floor { 1 } else { 2 }, 1);
            let entity = npcs::npc(
                &mut app,
                1,
                1,
                vec![],
                if floor { &[] } else { &[1, 1, 0, 0, 1] },
            );
            {
                let mut events = app.world_mut().resource_mut::<MapEvents>();
                let page = &mut events.events[0].pages[0];
                page.trigger = 2;
                page.layer = if floor { 0 } else { 1 };
                if !empty {
                    page.commands = vec![cmd(11410, 0, vec![100])];
                }
            }
            app.world_mut()
                .get_mut::<EventSprite>(entity)
                .unwrap()
                .layer = if floor { 0 } else { 1 };
            app.world_mut()
                .get_mut::<RouteStepper>(entity)
                .unwrap()
                .set_stop_count(77);
            app.update();
            assert_eq!(count(&app, entity), 1, "empty={empty}, floor={floor}");
            assert_eq!(app.world().resource::<RunningEvent>().active(), !empty);
        }
    }
}

#[test]
fn blocked_jumps_do_not_trigger_walk_failure_contacts_or_reset_the_stop_count() {
    let mut app = app();
    hero_at(&mut app, 2, 1);
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[1, 1, 0, 0, 24, 1, 25]);
    {
        let mut events = app.world_mut().resource_mut::<MapEvents>();
        events.events[0].pages[0].trigger = 2;
        events.events[0].pages[0].commands = vec![switch_cmd(7, 0, 0)];
    }
    app.update();
    assert!(!switch_on(&app, 7));
    assert_eq!(x(&app, entity), 1);
    assert_eq!(count(&app, entity), 65536);
}

#[test]
fn failed_diagonals_do_not_treat_their_destination_as_a_cardinal_front_contact() {
    let mut app = app();
    hero_at(&mut app, 2, 0);
    let entity = npcs::npc(&mut app, 1, 1, vec![], &[1, 1, 0, 0, 4]);
    {
        let mut events = app.world_mut().resource_mut::<MapEvents>();
        events.events[0].pages[0].trigger = 2;
        events.events[0].pages[0].commands = vec![switch_cmd(7, 0, 0)];
    }
    app.update();
    assert!(!switch_on(&app, 7));
    assert_eq!(x(&app, entity), 1);
    assert_eq!(count(&app, entity), 65536);
    assert!(!app.world().get::<MoveQueue>(entity).unwrap().busy());
}

#[derive(Resource, Default)]
struct Trace(Vec<(i32, u32, u32, bool)>);

#[test]
fn full_movement_and_stop_clock_traces_match_at_all_render_rates() {
    let mut reference = None;
    for fps in [15, 30, 60, 144] {
        let mut app = app();
        app.add_plugins((
            crate::timing::TimingPlugin,
            crate::timing::logical::LogicalPlugin,
        ));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / f64::from(fps),
        )));
        npcs::npc(&mut app, 1, 1, vec![], &[1, 7, 0, 0, 23, 1, 12, 1]);
        app.init_resource::<Trace>().add_systems(
            Update,
            (|events: Query<(&EventSprite, &RouteStepper, &MoveQueue)>,
              mut trace: ResMut<Trace>| {
                let (event, route, queue) = events.single().unwrap();
                trace.0.push((
                    event.tile_x,
                    route.stop_count(),
                    route.stop_maximum(),
                    queue.busy(),
                ));
            })
            .after(crate::interpreter::InterpreterStep),
        );
        for _ in 0..fps * 2 {
            app.update();
        }
        let actual = &app.world().resource::<Trace>().0;
        assert_eq!(actual.len(), 120, "{fps} FPS");
        assert_eq!(actual[0], (1, 1, 22, false));
        assert_eq!(actual[21], (1, 22, 22, false));
        assert_eq!(actual[22], (2, 0, 4, true));
        if let Some(reference) = &reference {
            assert_eq!(actual, reference, "{fps} FPS");
        } else {
            reference = Some(actual.clone());
        }
    }
}

#[test]
fn the_terminal_async_frame_holds_hero_routes_and_vehicle_clocks() {
    use crate::vehicles::{VehiclePlugin, Vehicles};
    let mut app = app();
    app.add_plugins(VehiclePlugin)
        .init_resource::<crate::audio::CurrentBgm>();
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_location(0, 0, 4, 4);
    let hero = {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<Player>>()
            .single(world)
            .unwrap()
    };
    app.world_mut()
        .get_mut::<RouteStepper>(hero)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[10001, 7, 0, 0, 23, 1]));
    let route = app.world().get::<RouteStepper>(hero).unwrap().clone();
    let vehicles = app.world().resource::<Vehicles>().motion_snapshot();
    app.insert_resource(crate::timing::SceneWait(true));
    app.update();
    assert_eq!(app.world().get::<RouteStepper>(hero).unwrap(), &route);
    assert_eq!(
        app.world().resource::<Vehicles>().motion_snapshot(),
        vehicles
    );
    app.world_mut().resource_mut::<crate::timing::SceneWait>().0 = false;
    app.update();
    assert_eq!(count(&app, hero), 1);
    assert_ne!(
        app.world().resource::<Vehicles>().motion_snapshot(),
        vehicles
    );
}
