use super::*;

#[test]
fn a_relocated_vehicles_zero_remaining_jump_survives_an_actual_save_file() {
    let (mut app, path) = app("relocation");
    app.world_mut().resource_scope(|world, data: Mut<MapData>| {
        let mut vehicles = world.resource_mut::<Vehicles>();
        start(&mut vehicles, &data, 0, &[24, 1, 1, 2, 25, 23, 1]);
        vehicles.set_location(0, data.map_id, 4, 6);
    });
    let state = app.world().resource::<Vehicles>().save.clone();
    let motion = app.world().resource::<Vehicles>().motion_snapshot();
    assert!(motion.valid(&state));
    save_and_load(&mut app);
    let vehicles = app.world().resource::<Vehicles>();
    assert_eq!(vehicles.save, state);
    assert_eq!(vehicles.motion_snapshot(), motion);
    assert!(vehicles.motion[0].queue.jumping());
    let data = app.world().resource::<MapData>();
    assert_eq!(
        vehicles.motion[0]
            .queue
            .render_position(&vehicles.save.vehicles[0], data),
        Vec2::from(data.tile_center(4, 6))
    );
    std::fs::remove_file(path).unwrap();
}
