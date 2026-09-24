use super::*;

#[test]
fn a_real_slot_retains_pending_autoruns_through_loading_and_resumption() {
    let (mut app, path) = app("queued");
    app.world_mut().resource_mut::<MapData>().map_id = 3;
    let mut events = vec![];
    for id in 1..=2 {
        let mut commands = vec![
            cmd(10220, 0, vec![0, 1, 1, 3, 0, 10]),
            cmd(10220, 0, vec![0, 1, 1, 1, 0, id as i32]),
            switch_cmd(id as i32 + 4, 1, 0),
        ];
        if id == 1 {
            commands.insert(0, cmd(11910, 0, vec![]));
        }
        let mut event = map_event(id, 3, commands);
        event.pages[0].condition.flags = 1;
        event.pages[0].condition.switch_a = id + 4;
        events.push(event);
        app.world_mut().resource_mut::<Switches>().set(id + 4, true);
    }
    app.insert_resource(MapEvents { events });
    app.update();
    let saved = app.world().resource::<RunningEvent>().snapshot().unwrap();
    app.update();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(app.world().resource::<Variables>().get(1), 12);
    load(&mut app);
    assert_eq!(
        app.world().resource::<RunningEvent>().snapshot(),
        Some(saved.clone())
    );
    for _ in 0..4 {
        app.update();
        assert_eq!(
            app.world().resource::<RunningEvent>().snapshot(),
            Some(saved.clone())
        );
        assert_eq!(app.world().resource::<Variables>().get(1), 0);
    }
    resume(&mut app);
    assert_eq!(app.world().resource::<Variables>().get(1), 12);
    assert!(app.world().resource::<RunningEvent>().snapshot().is_none());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_file(path).unwrap();
}
