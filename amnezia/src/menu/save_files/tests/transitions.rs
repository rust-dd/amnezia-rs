use super::*;
use crate::save::EventSaveRequest;
use crate::transitions::Transition;

struct Fixture {
    app: App,
    directory: std::path::PathBuf,
}

impl Fixture {
    fn new(tag: &str, crystal: bool) -> Self {
        let (mut app, directory) = app(tag);
        if crystal {
            app.world_mut().resource_mut::<MenuOpen>().0 = false;
            app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
        }
        Self { app, directory }
    }

    fn start(&mut self, crystal: bool) {
        step(&mut self.app, (!crystal).then_some(KeyCode::Enter));
    }

    fn open(&mut self, crystal: bool) {
        self.start(crystal);
        for _ in 0..12 {
            step(&mut self.app, None);
        }
        assert!(!self.app.world().resource::<Transition>().busy());
        assert!(self.app.world().resource::<SaveFiles>().active());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for number in 1..=15 {
            let path = self.directory.join(format!("slot{number}.ron"));
            if path.is_file() {
                std::fs::remove_file(path).unwrap();
            }
        }
        std::fs::remove_dir(&self.directory).unwrap();
    }
}

#[test]
fn both_save_origins_erase_the_parent_then_show_the_populated_list_for_six_frames() {
    for crystal in [false, true] {
        let mut fixture = Fixture::new(&format!("save_scene_entry_{crystal}"), crystal);
        fixture.start(crystal);
        let app = &mut fixture.app;
        assert!(app.world().resource::<Transition>().busy());
        assert!(app.world().resource::<SaveFiles>().entries.is_none());
        for _ in 0..5 {
            step(app, Some(KeyCode::Enter));
            assert!(app.world().resource::<SaveFiles>().entries.is_none());
        }
        step(app, None);
        assert!(app.world().resource::<Transition>().busy());
        assert!(app.world().resource::<SaveFiles>().entries.is_some());
        for _ in 0..5 {
            step(app, Some(KeyCode::Enter));
            assert!(app.world().resource::<Transition>().busy());
            assert!(app.world().resource::<SaveFiles>().decision().is_none());
        }
        step(app, None);
        assert!(!app.world().resource::<Transition>().busy());
        assert_eq!(std::fs::read_dir(&fixture.directory).unwrap().count(), 0);
    }
}

#[test]
fn cancelling_keeps_the_list_until_erased_and_returns_to_the_correct_parent_after_showing() {
    for crystal in [false, true] {
        let mut fixture = Fixture::new(&format!("save_scene_cancel_{crystal}"), crystal);
        fixture.open(crystal);
        let app = &mut fixture.app;
        step(app, Some(KeyCode::Escape));
        assert!(app.world().resource::<Transition>().busy());
        assert!(app.world().resource::<SaveFiles>().entries.is_some());
        for _ in 0..5 {
            step(app, Some(KeyCode::Enter));
            assert!(app.world().resource::<SaveFiles>().entries.is_some());
        }
        step(app, None);
        assert!(app.world().resource::<Transition>().busy());
        assert!(app.world().resource::<SaveFiles>().entries.is_none());
        assert_eq!(app.world().resource::<MenuOpen>().0, !crystal);
        for _ in 0..5 {
            step(app, Some(KeyCode::Escape));
            assert_eq!(app.world().resource::<MenuOpen>().0, !crystal);
        }
        step(app, None);
        assert!(!app.world().resource::<Transition>().busy());
        assert!(!app.world().resource::<SaveFiles>().active());
        assert_eq!(app.world().resource::<MenuState>().cursor, 3);
        assert!(!app.world().resource::<EventSaveRequest>().0);
        assert_eq!(std::fs::read_dir(&fixture.directory).unwrap().count(), 0);
    }
}

#[test]
fn confirmation_writes_once_before_the_list_exit_fade_and_blocks_input_until_return() {
    for crystal in [false, true] {
        let mut fixture = Fixture::new(&format!("save_scene_confirm_{crystal}"), crystal);
        fixture.open(crystal);
        let app = &mut fixture.app;
        step(app, Some(KeyCode::Enter));
        step(app, None);
        let path = fixture.directory.join("slot1.ron");
        let saved = std::fs::read(&path).unwrap();
        assert!(app.world().resource::<Transition>().busy());
        assert!(app.world().resource::<SaveFiles>().entries.is_some());
        for _ in 0..5 {
            step(app, Some(KeyCode::Enter));
            assert_eq!(std::fs::read(&path).unwrap(), saved);
        }
        step(app, None);
        assert!(app.world().resource::<Transition>().busy());
        assert!(app.world().resource::<SaveFiles>().entries.is_none());
        for _ in 0..6 {
            step(app, None);
            assert_eq!(std::fs::read(&path).unwrap(), saved);
        }
        assert!(!app.world().resource::<SaveFiles>().active());
        assert_eq!(app.world().resource::<MenuOpen>().0, !crystal);
    }
}

#[test]
fn render_updates_without_logical_ticks_do_not_advance_entry_or_file_clocks() {
    let mut fixture = Fixture::new("save_scene_clock_hold", false);
    fixture.start(false);
    let app = &mut fixture.app;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    for _ in 0..40 {
        app.update();
        assert_eq!(app.world().resource::<Transition>().age(), 0);
        assert!(app.world().resource::<SaveFiles>().entries.is_none());
    }
    advance(app, 6);
    for _ in 0..40 {
        app.update();
        let files = app.world().resource::<SaveFiles>();
        assert_eq!(files.navigation.cursors[0], 1);
        assert_eq!(files.navigation.arrow, 0);
        assert_eq!(files.navigation.arrows, [false; 2]);
        assert_eq!(app.world().resource::<Transition>().age(), 0);
    }
    advance(app, 6);
    assert!(!app.world().resource::<Transition>().busy());
}

#[test]
fn real_frame_clock_keeps_the_save_before_the_exit_transition() {
    for crystal in [false, true] {
        let mut fixture = Fixture::new(&format!("save_scene_real_clock_{crystal}"), crystal);
        let app = &mut fixture.app;
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
        if !crystal {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Enter);
        }
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        for _ in 0..12 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<crate::timing::GameFrames>().frame,
            12
        );
        assert!(!app.world().resource::<Transition>().busy());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert_eq!(app.world().resource::<SaveFiles>().decision(), Some(true));
        assert!(!fixture.directory.join("slot1.ron").exists());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
        assert!(fixture.directory.join("slot1.ron").is_file());
        assert_eq!(app.world().resource::<Transition>().age(), 0);
        assert!(app.world().resource::<Transition>().busy());
        for _ in 0..12 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<crate::timing::GameFrames>().frame,
            26
        );
        assert!(!app.world().resource::<SaveFiles>().active());
        assert_eq!(app.world().resource::<MenuOpen>().0, !crystal);
    }
}

#[test]
fn both_approved_save_origins_wait_for_an_intervening_transition() {
    for crystal in [false, true] {
        let mut fixture = Fixture::new(&format!("save_scene_deferred_write_{crystal}"), crystal);
        fixture.open(crystal);
        let app = &mut fixture.app;
        step(app, Some(KeyCode::Enter));
        let now = app.world().resource::<crate::timing::GameFrames>().frame;
        app.world_mut().resource_mut::<Transition>().start_for(
            crate::transitions::Kind::Fade,
            false,
            now,
            IVec2::ZERO,
            6,
        );
        for _ in 0..6 {
            step(app, None);
            assert!(!fixture.directory.join("slot1.ron").exists());
        }
        step(app, None);
        assert!(fixture.directory.join("slot1.ron").is_file());
        assert!(app.world().resource::<Transition>().busy());
        advance(app, 12);
        assert!(!app.world().resource::<SaveFiles>().active());
    }
}
