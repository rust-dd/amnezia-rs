use super::*;
use crate::vehicles::Vehicles;

#[test]
fn airship_minimap_condition_is_false_when_walking() {
    let mut app = interp_app();
    let commands = vec![
        cmd(12010, 0, vec![7, 2, 0, 0, 0, 1]),
        switch_cmd(610, 0, 1),
        cmd(22011, 0, vec![]),
    ];
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, commands.clone());
    app.update();
    assert!(!switch_on(&app, 610));
    app.world_mut().resource_mut::<Vehicles>().save.riding = Some(2);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, commands);
    app.update();
    assert!(switch_on(&app, 610));
}

#[test]
fn locate_board_and_query_airship_in_one_interpreter_frame() {
    let mut app = interp_app();
    let mut data = MapData::for_test(100, 110);
    data.map_id = 13;
    app.insert_resource(data);
    let world = app.world_mut();
    let mut hero = world.query::<&mut Player>().single_mut(world).unwrap();
    hero.tile_x = 55;
    hero.tile_y = 100;
    app.world_mut().resource_mut::<Variables>().set(48, 13);
    app.world_mut().resource_mut::<Variables>().set(49, 55);
    app.world_mut().resource_mut::<Variables>().set(50, 100);
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            cmd(10850, 0, vec![2, 1, 48, 49, 50]),
            cmd(10840, 0, vec![]),
            cmd(12010, 0, vec![6, 10004, 3]),
            switch_cmd(800, 0, 1),
            cmd(22011, 0, vec![]),
            cmd(11330, 0, vec![10004, 8, 0, 0, 3, 3]),
        ],
    );
    app.update();
    let vehicles = app.world().resource::<Vehicles>();
    assert!(vehicles.riding());
    assert!(vehicles.moving());
    assert_eq!(
        vehicles.character(10004),
        Some((55, 100, crate::tiles::DIR_LEFT))
    );
    assert!(app.world().resource::<Switches>().get(800));
}
