use super::*;
use crate::save::{SaveLocation, SavePlugin};

fn app(tag: &str) -> (App, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "amnezia-save-boundary-{tag}-{}.ron",
        std::process::id()
    ));
    let mut app = interp_app();
    app.add_plugins((AssetPlugin::default(), SavePlugin))
        .init_asset::<Image>()
        .init_resource::<crate::screenfx::TintState>()
        .init_resource::<crate::timer::PlayTime>()
        .insert_resource(SaveLocation(path.clone()));
    (app, path)
}

#[derive(serde::Deserialize)]
struct SavedValues {
    switches: Vec<(u32, bool)>,
    variables: Vec<(u32, i32)>,
    save_access: bool,
}

#[test]
fn the_file_contains_only_values_from_before_the_save_command() {
    let (mut app, path) = app("values");
    assert!(!path.exists());
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            switch_cmd(901, 2, 0),
            cmd(10220, 0, vec![0, 1, 1, 0, 0, 42]),
            cmd(11930, 0, vec![1]),
            cmd(11910, 0, vec![]),
            switch_cmd(902, 2, 0),
            cmd(10220, 0, vec![0, 1, 1, 0, 0, 99]),
            cmd(11930, 0, vec![0]),
        ],
    );
    app.update();
    assert!(!path.exists());
    assert!(app.world().resource::<EventSaveRequest>().0);
    assert_eq!(app.world().resource::<Variables>().get(1), 42);
    app.update();
    let bytes = std::fs::read_to_string(&path).unwrap();
    let saved = ron::from_str::<SavedValues>(&bytes).unwrap();
    assert!(saved.switches.contains(&(901, true)));
    assert!(!saved.switches.contains(&(902, true)));
    assert!(saved.variables.contains(&(1, 42)));
    assert!(saved.save_access);
    assert_eq!(app.world().resource::<Variables>().get(1), 99);
    assert!(switch_on(&app, 902));
    assert!(!app.world().resource::<SaveAccess>().0);
    assert!(!app.world().resource::<RunningEvent>().active());
    assert!(!app.world().resource::<EventSaveRequest>().0);
    for _ in 0..4 {
        app.update();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), bytes);
        assert!(switch_on(&app, 901));
        assert!(switch_on(&app, 902));
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_failed_file_write_does_not_leave_the_event_stuck_or_overwrite_the_destination() {
    let (mut app, path) = app("failed");
    std::fs::create_dir(&path).unwrap();
    let existing = path.join("existing.ron");
    std::fs::write(&existing, "untouched").unwrap();
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, vec![cmd(11910, 0, vec![]), switch_cmd(901, 2, 0)]);
    app.update();
    assert!(app.world().resource::<EventSaveRequest>().0);
    assert!(!switch_on(&app, 901));
    for _ in 0..4 {
        app.update();
        assert!(!app.world().resource::<EventSaveRequest>().0);
        assert!(!app.world().resource::<RunningEvent>().active());
        assert!(switch_on(&app, 901));
        assert_eq!(std::fs::read_to_string(&existing).unwrap(), "untouched");
    }
    std::fs::remove_file(existing).unwrap();
    std::fs::remove_dir(path).unwrap();
}

#[test]
fn a_transition_defers_the_file_write_and_the_event_tail_together() {
    let (mut app, path) = app("deferred");
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, vec![cmd(11910, 0, vec![]), switch_cmd(901, 2, 0)]);
    app.update();
    assert!(
        app.world_mut()
            .resource_mut::<crate::transitions::Transition>()
            .start(
                crate::transitions::Kind::Fade,
                true,
                0,
                IVec2::new(160, 120),
            )
    );
    for _ in 0..4 {
        app.update();
        assert!(!path.exists());
        assert!(app.world().resource::<EventSaveRequest>().0);
        assert!(app.world().resource::<RunningEvent>().active());
        assert!(!switch_on(&app, 901));
    }
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    app.update();
    let saved = ron::from_str::<SavedValues>(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(!saved.switches.contains(&(901, true)));
    assert!(switch_on(&app, 901));
    assert!(!app.world().resource::<EventSaveRequest>().0);
    assert!(!app.world().resource::<RunningEvent>().active());
    std::fs::remove_file(path).unwrap();
}
