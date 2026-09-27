use super::*;
use amnezia_data::EventCommand;

#[test]
fn transient_route_switches_clear_old_parallel_owners_for_npcs_hero_and_all_vehicles() {
    for target in [2, 10001, 10002, 10003, 10004] {
        let mut parallel = page(vec![]);
        parallel.trigger = 4;
        parallel.commands = vec![
            EventCommand {
                code: 11410,
                indent: 0,
                string: String::new(),
                params: vec![10],
            },
            EventCommand {
                code: 10210,
                indent: 0,
                string: String::new(),
                params: vec![0, 9, 9, 0],
            },
        ];
        let mut app = app(
            vec![
                event(1, 1, vec![parallel, gated(page(vec![]))]),
                event(2, 3, vec![page(vec![])]),
            ],
            false,
        );
        app.init_resource::<crate::audio::CurrentBgm>()
            .init_resource::<crate::system_bgm::SystemBgm>()
            .add_plugins(crate::vehicles::VehiclePlugin);
        let map_id = app.world().resource::<MapData>().map_id;
        for index in 0..3 {
            app.world_mut()
                .resource_mut::<crate::vehicles::Vehicles>()
                .set_location(index, map_id, 7 + index as u32, 8);
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<crate::interpreter::ParallelPool>()
                .count(),
            1
        );
        let route = RouteStepper::from_move_event(&[target, 8, 0, 0, 32, 7, 33, 7]);
        match target {
            2 => {
                let npc = entity(&mut app, 2);
                app.world_mut()
                    .get_mut::<RouteStepper>(npc)
                    .unwrap()
                    .force_route(route);
            }
            10001 => {
                let world = app.world_mut();
                world
                    .query_filtered::<&mut RouteStepper, With<Player>>()
                    .single_mut(world)
                    .unwrap()
                    .force_route(route);
            }
            _ => app
                .world_mut()
                .resource_mut::<crate::vehicles::Vehicles>()
                .set_route(target, route),
        }
        app.update();
        assert!(!app.world().resource::<Switches>().get(7));
        assert!(!app.world().resource::<Switches>().get(9));
        assert_eq!(
            app.world()
                .resource::<crate::interpreter::ParallelPool>()
                .count(),
            0,
            "target {target}"
        );
        app.update();
        assert_eq!(
            app.world()
                .resource::<crate::interpreter::ParallelPool>()
                .count(),
            1
        );
        assert!(!app.world().resource::<Switches>().get(9));
    }
}

#[test]
fn a_route_page_pulse_cancels_an_already_queued_foreground_event() {
    let mut autorun = page(vec![]);
    autorun.trigger = 3;
    autorun.commands = vec![EventCommand {
        code: 10210,
        indent: 0,
        string: String::new(),
        params: vec![0, 9, 9, 0],
    }];
    let mut app = app(
        vec![
            event(1, 1, vec![autorun, gated(page(vec![]))]),
            event(2, 3, vec![page(vec![command(32, 7), command(33, 7)])]),
        ],
        false,
    );
    app.update();
    assert!(!app.world().resource::<Switches>().get(9));
    assert!(
        app.world()
            .resource::<crate::interpreter::RunningEvent>()
            .queued_ids()
            .is_empty()
    );
}

#[test]
fn a_parallel_page_disappearing_in_its_script_still_finishes_its_character_update_once() {
    let mut parallel = gated(page(vec![]));
    parallel.trigger = 4;
    parallel.commands = vec![EventCommand {
        code: 10210,
        indent: 0,
        string: String::new(),
        params: vec![0, 7, 7, 1],
    }];
    let mut app = app(vec![event(1, 1, vec![parallel])], true);
    let npc = entity(&mut app, 1);
    app.update();
    assert_eq!(
        app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
        1
    );
    app.update();
    assert_eq!(
        app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
        1
    );
}
