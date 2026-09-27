use super::*;

#[test]
fn occupied_vehicle_relocation_updates_the_hero_before_the_next_script_command() {
    for boarding in [false, true] {
        let (mut app, hero) = app();
        if boarding {
            test_support::direction(&mut app, DIR_RIGHT);
            app.world_mut()
                .resource_mut::<Vehicles>()
                .set_location(0, 0, 6, 5);
            test_support::toggle(&mut app);
            assert!(app.world().get::<MoveQueue>(hero).unwrap().busy());
            assert!(app.world().resource::<Vehicles>().save.boarding);
        } else {
            rider(&mut app, 0);
        }
        let cmd = |code, params| amnezia_data::EventCommand {
            code,
            params,
            indent: 0,
            string: String::new(),
        };
        let commands = vec![
            cmd(10850, vec![0, 0, 0, 8, 9]),
            cmd(10220, vec![0, 1, 1, 0, 6, 10001, 1]),
            cmd(10220, vec![0, 2, 2, 0, 6, 10001, 2]),
            cmd(10220, vec![0, 3, 3, 0, 6, 10002, 1]),
        ];
        if boarding {
            app.insert_resource(crate::interpreter::CommonEvents(vec![
                amnezia_data::CommonEvent {
                    id: 1,
                    name: String::new(),
                    trigger: 4,
                    switch_flag: false,
                    switch_id: 0,
                    commands,
                },
            ]));
        } else {
            app.world_mut()
                .resource_mut::<crate::interpreter::RunningEvent>()
                .start(0, commands);
        }
        app.update();
        let variables = app.world().resource::<Variables>();
        assert_eq!(
            (variables.get(1), variables.get(2), variables.get(3)),
            (8, 9, 8)
        );
        assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (8, 9));
        assert!(!app.world().resource::<crate::teleport::Fade>().busy());
    }
}

#[test]
fn occupied_vehicle_relocation_keeps_the_hero_route_and_resets_only_its_party_graphic_and_opacity()
{
    let (mut app, hero) = app();
    app.add_plugins((
        crate::gamedata::GameDataPlugin,
        crate::appearance::AppearancePlugin,
    ));
    app.update();
    let graphic = app.world().get::<Player>(hero).unwrap().charset.clone();
    rider(&mut app, 0);
    script(&mut app, 10001, &[29, 40, 1, 32, 9]);
    app.update();
    app.update();
    assert!(app.world().get::<MoveQueue>(hero).unwrap().busy());
    app.world_mut()
        .get_mut::<Player>(hero)
        .unwrap()
        .set_graphic("Poses2".into(), 4);
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_location(0, 0, 8, 9);
    app.world_mut().resource_mut::<Vehicles>().relocate_pending = Some((8, 9));
    flush(app.world_mut());
    assert_eq!(app.world().get::<Player>(hero).unwrap().charset, graphic);
    let route = app.world().get::<RouteStepper>(hero).unwrap();
    assert_eq!(route.alpha(), 1.0);
    assert_eq!(route.speed(), 3);
    assert!(route.pending());
    assert!(!app.world().get::<MoveQueue>(hero).unwrap().busy());
    app.update();
    assert!(app.world().resource::<Switches>().get(9));
}
