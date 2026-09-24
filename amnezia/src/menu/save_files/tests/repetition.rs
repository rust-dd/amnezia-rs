use super::*;
use crate::audio::{AudioRequest, SystemSounds};
use crate::timing::GameFrames;

struct Fixture {
    app: App,
    directory: std::path::PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let (mut app, directory) = app(tag);
        app.insert_resource(SystemSounds {
            cursor: amnezia_data::SoundDef {
                name: "CURSOR".into(),
                volume: 100,
                tempo: 100,
                ..default()
            },
            ..default()
        });
        app.world_mut().resource_mut::<MenuOpen>().0 = false;
        app.update();
        Self { app, directory }
    }

    fn open(&mut self) {
        self.app
            .world_mut()
            .resource_scope(|world, mut files: Mut<SaveFiles>| {
                files.open_load(
                    world.resource::<SaveLocation>(),
                    world.resource::<GameData>(),
                    world.resource::<GameFrames>().frame,
                );
            });
    }

    fn tick(&mut self) -> Vec<String> {
        self.app.world_mut().resource_mut::<GameFrames>().frame += 1;
        self.app.update();
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        self.app
            .world_mut()
            .resource_mut::<Messages<AudioRequest>>()
            .drain()
            .map(|request| {
                let AudioRequest::Sound { name, .. } = request else {
                    panic!("unexpected file audio")
                };
                name
            })
            .collect()
    }

    fn index(&self) -> usize {
        self.app.world().resource::<SaveFiles>().navigation.index
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(std::fs::read_dir(&self.directory).unwrap().count(), 0);
        std::fs::remove_dir(&self.directory).unwrap();
    }
}

#[test]
fn file_navigation_inherits_held_arrow_phase_across_opening_and_fades() {
    for paused in [false, true] {
        let mut fixture = Fixture::new(&format!("file_shared_repeat_{paused}"));
        if paused {
            fixture.open();
            fixture
                .app
                .world_mut()
                .resource_mut::<crate::transitions::Transition>()
                .start_for(crate::transitions::Kind::Fade, false, 0, IVec2::ZERO, 1000);
        }
        fixture
            .app
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        for _ in 0..23 {
            assert!(fixture.tick().is_empty());
        }
        if paused {
            fixture
                .app
                .world_mut()
                .resource_mut::<crate::transitions::Transition>()
                .clear();
        } else {
            fixture.open();
        }
        assert_eq!(fixture.tick(), ["CURSOR"], "paused: {paused}");
        assert_eq!(fixture.index(), 1);
        for _ in 0..3 {
            assert!(fixture.tick().is_empty());
            assert_eq!(fixture.index(), 1);
        }
        assert_eq!(fixture.tick(), ["CURSOR"]);
        assert_eq!(fixture.index(), 2);
    }
}

#[test]
fn simultaneous_file_directions_sound_individually_even_if_selection_returns_to_the_start() {
    let mut fixture = Fixture::new("file_shared_simultaneous");
    fixture.open();
    let mut keys = fixture
        .app
        .world_mut()
        .resource_mut::<ButtonInput<KeyCode>>();
    for key in [
        KeyCode::ArrowDown,
        KeyCode::ArrowUp,
        KeyCode::PageDown,
        KeyCode::PageUp,
    ] {
        keys.press(key);
    }
    assert_eq!(fixture.tick(), ["CURSOR"; 4]);
    assert_eq!(fixture.index(), 0);
}
