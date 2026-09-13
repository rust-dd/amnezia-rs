use super::*;

#[test]
fn saved_erasure_does_not_survive_as_a_map_scene_flag_after_loading() {
    for load_from_black in [false, true] {
        let (mut app, path) = app(&format!("erasure-{load_from_black}"));
        let mut transition = app
            .world_mut()
            .resource_mut::<crate::transitions::Transition>();
        transition.hold_black();
        transition.event_erased = true;
        app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
        app.update();
        let bytes = std::fs::read_to_string(&path).unwrap();
        assert!(!bytes.contains("event_erased"));
        if !load_from_black {
            app.world_mut()
                .resource_mut::<crate::transitions::Transition>()
                .clear();
        }
        load(&mut app);
        assert!(
            !app.world()
                .resource::<crate::transitions::Transition>()
                .event_erased
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), bytes);
        std::fs::remove_file(path).unwrap();
    }
}
