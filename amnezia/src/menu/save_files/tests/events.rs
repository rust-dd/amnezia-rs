use super::*;
use crate::save::EventSaveRequest;

#[test]
fn a_crystal_request_waits_for_the_selector_without_writing_or_enabling_manual_saves() {
    let (mut app, directory) = app("crystal_selector_wait");
    app.world_mut().resource_mut::<MenuOpen>().0 = false;
    app.world_mut().resource_mut::<SaveAccess>().0 = false;
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    step(&mut app, None);
    assert_eq!(
        std::fs::read_dir(&directory).unwrap().count(),
        0,
        "opening a crystal's save menu must not write a slot"
    );
    assert!(app.world().resource::<EventSaveRequest>().0);
    assert!(app.world().resource::<SaveFiles>().active());
    assert!(!app.world().resource::<SaveAccess>().0);
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn opening_a_crystal_selector_clears_an_event_erased_scene() {
    let (mut app, directory) = app("crystal_selector_erased");
    let mut transition = crate::transitions::Transition::default();
    transition.hold_black();
    transition.event_erased = true;
    app.insert_resource(transition);
    app.world_mut().resource_mut::<MenuOpen>().0 = false;
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    step(&mut app, None);
    assert!(app.world().resource::<SaveFiles>().active());
    let transition = app.world().resource::<crate::transitions::Transition>();
    assert!(!transition.erased() && !transition.event_erased);
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn a_failed_selected_write_preserves_the_destination_and_closes_the_crystal_menu() {
    let (mut app, directory) = app("crystal_selector_failure");
    let destination = directory.join("slot1.ron");
    std::fs::create_dir(&destination).unwrap();
    let marker = destination.join("keep.ron");
    std::fs::write(&marker, "unchanged").unwrap();
    app.world_mut().resource_mut::<MenuOpen>().0 = false;
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    step(&mut app, None);
    advance(&mut app, 12);
    step(&mut app, Some(KeyCode::Enter));
    step(&mut app, None);
    advance(&mut app, 12);
    assert!(!app.world().resource::<SaveFiles>().active());
    assert!(!app.world().resource::<MenuOpen>().0);
    assert!(!app.world().resource::<EventSaveRequest>().0);
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "unchanged");
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    std::fs::remove_file(marker).unwrap();
    std::fs::remove_dir(destination).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn a_transition_defers_opening_and_the_confirmed_event_write() {
    let (mut app, directory) = app("crystal_selector_transition");
    let mut transition = crate::transitions::Transition::default();
    transition.start(
        crate::transitions::Kind::Fade,
        true,
        0,
        IVec2::new(160, 120),
    );
    app.insert_resource(transition);
    app.world_mut().resource_mut::<MenuOpen>().0 = false;
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    step(&mut app, None);
    assert!(!app.world().resource::<SaveFiles>().active());
    assert!(app.world().resource::<EventSaveRequest>().0);
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    step(&mut app, None);
    assert!(app.world().resource::<SaveFiles>().active());
    advance(&mut app, 12);
    step(&mut app, Some(KeyCode::Enter));
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .start(
            crate::transitions::Kind::Fade,
            true,
            0,
            IVec2::new(160, 120),
        );
    step(&mut app, None);
    assert!(app.world().resource::<SaveFiles>().active());
    assert!(!directory.join("slot1.ron").exists());
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    step(&mut app, None);
    assert!(app.world().resource::<SaveFiles>().active());
    assert!(directory.join("slot1.ron").is_file());
    advance(&mut app, 12);
    assert!(!app.world().resource::<SaveFiles>().active());
    std::fs::remove_file(directory.join("slot1.ron")).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn session_cleanup_discards_a_pending_crystal_selector_without_writing() {
    let (mut app, directory) = app("crystal_selector_reset");
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    step(&mut app, None);
    assert!(app.world().resource::<SaveFiles>().active());
    crate::session::clear_transient(app.world_mut());
    assert!(!app.world().resource::<SaveFiles>().active());
    step(&mut app, None);
    assert!(!app.world().resource::<MenuOpen>().0);
    assert!(!app.world().resource::<EventSaveRequest>().0);
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
    std::fs::remove_dir(directory).unwrap();
}
