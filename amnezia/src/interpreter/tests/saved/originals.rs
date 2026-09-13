use super::*;

#[test]
fn all_original_crystal_pages_resume_at_their_end_without_replaying_the_save_sound() {
    let (mut app, path) = app("all-crystals");
    let mut crystals = 0;
    for entry in std::fs::read_dir(format!("{}/maps", crate::assets::asset_root())).unwrap() {
        let source = entry.unwrap().path();
        if source
            .extension()
            .is_none_or(|extension| extension != "ron")
        {
            continue;
        }
        let map = crate::assets::load_ron::<amnezia_data::Map>(source.to_str().unwrap());
        for event in &map.events {
            for page in &event.pages {
                let Some(save) = page
                    .commands
                    .iter()
                    .position(|command| command.code == 11910)
                else {
                    continue;
                };
                app.world_mut()
                    .resource_mut::<RunningEvent>()
                    .start(event.id, page.commands.clone());
                app.update();
                let before = app.world().resource::<RunningEvent>().frame.clone();
                assert!(before.active());
                assert_eq!(before.ip, save + 1);
                app.update();
                let bytes = std::fs::read(&path).unwrap();
                load(&mut app);
                assert_eq!(app.world().resource::<RunningEvent>().frame, before);
                resume(&mut app);
                assert!(!app.world().resource::<RunningEvent>().active());
                assert!(!app.world().resource::<EventSaveRequest>().0);
                assert_eq!(std::fs::read(&path).unwrap(), bytes);
                for sound in app
                    .world_mut()
                    .resource_mut::<Messages<AudioRequest>>()
                    .drain()
                {
                    assert!(!matches!(sound, AudioRequest::Sound { .. }));
                }
                crystals += 1;
            }
        }
    }
    assert_eq!(crystals, 16);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn parallel_pages_restart_from_the_selected_page_instead_of_a_saved_instruction_pointer() {
    for changed_page in [false, true] {
        let (mut app, path) = app(&format!("parallel-{changed_page}"));
        let mut event = map_event(
            2,
            4,
            vec![
                cmd(10220, 0, vec![0, 10, 10, 1, 0, 1]),
                cmd(11410, 0, vec![100]),
                switch_cmd(902, 0, 0),
            ],
        );
        let mut alternate = event.pages[0].clone();
        alternate.condition.flags = 1;
        alternate.condition.switch_a = 901;
        alternate.commands[0].params[5] = 10;
        event.pages.push(alternate);
        app.insert_resource(MapEvents {
            events: vec![event],
        });
        app.update();
        assert_eq!(app.world().resource::<Variables>().get(10), 1);
        app.world_mut()
            .resource_mut::<Switches>()
            .set(901, changed_page);
        app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
        app.update();
        load(&mut app);
        assert_eq!(app.world().resource::<Variables>().get(10), 1);
        assert!(!app.world().resource::<RunningEvent>().active());
        resume(&mut app);
        assert_eq!(
            app.world().resource::<Variables>().get(10),
            if changed_page { 11 } else { 2 }
        );
        assert!(!switch_on(&app, 902));
        std::fs::remove_file(path).unwrap();
    }
}
