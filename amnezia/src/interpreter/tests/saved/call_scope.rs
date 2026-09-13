use super::*;

fn option(index: i32) -> EventCommand {
    let mut command = cmd(20140, 0, vec![index]);
    command.string = index.to_string();
    command
}

fn select(app: &mut App, index: i32) {
    assert!(app.world().resource::<Choice>().active());
    app.world_mut().resource_mut::<Choice>().active = false;
    app.world_mut().resource_mut::<Choice>().result = Some(index);
    app.update();
}

#[test]
fn saving_inside_a_called_choice_preserves_both_distinct_selections() {
    let (mut app, path) = app("nested-choice");
    app.insert_resource(MapEvents {
        events: vec![map_event(
            2,
            0,
            vec![
                cmd(10140, 0, vec![0]),
                option(0),
                switch_cmd(902, 2, 1),
                option(1),
                switch_cmd(903, 2, 1),
                cmd(11910, 1, vec![]),
                switch_cmd(904, 2, 1),
                cmd(20141, 0, vec![]),
            ],
        )],
    });
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            cmd(10140, 0, vec![0]),
            option(0),
            switch_cmd(901, 2, 1),
            cmd(12330, 1, vec![1, 2, 1]),
            cmd(11410, 1, vec![0]),
            switch_cmd(905, 2, 1),
            option(1),
            switch_cmd(906, 2, 1),
            cmd(20141, 0, vec![]),
        ],
    );
    app.update();
    select(&mut app, 0);
    select(&mut app, 1);
    let before = app.world().resource::<RunningEvent>().frame.clone();
    assert_eq!(before.choices.get(&0), Some(&1));
    assert_eq!(before.call_stack[0].choices.get(&0), Some(&0));
    app.update();
    let bytes = std::fs::read(&path).unwrap();
    load(&mut app);
    assert_eq!(app.world().resource::<RunningEvent>().frame, before);
    resume(&mut app);
    assert_eq!(
        app.world().resource::<RunningEvent>().frame.choices.get(&0),
        Some(&0)
    );
    app.update();
    for id in [901, 903, 904, 905] {
        assert!(switch_on(&app, id));
    }
    for id in [902, 906] {
        assert!(!switch_on(&app, id));
    }
    assert!(!app.world().resource::<RunningEvent>().active());
    assert!(!app.world().resource::<Choice>().active());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_file(path).unwrap();
}
