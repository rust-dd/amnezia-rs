use super::{PathBuf, SaveLocation};
use bevy::prelude::*;

pub(super) struct Slot {
    directory: PathBuf,
    path: PathBuf,
    original_location: PathBuf,
}

impl Slot {
    pub(super) fn new(app: &mut App, scenario: &str) -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "amnezia-smoke-{scenario}-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("slot.ron");
        let original_location = app.world().resource::<SaveLocation>().0.clone();
        assert_ne!(path, original_location);
        app.insert_resource(SaveLocation(path.clone()));
        Self {
            directory,
            path,
            original_location,
        }
    }

    pub(super) fn path(&self, world: &World) -> PathBuf {
        assert_eq!(world.resource::<SaveLocation>().0, self.path);
        self.path.clone()
    }

    pub(super) fn finish(self, world: &mut World) {
        assert_eq!(world.resource::<SaveLocation>().0, self.path);
        std::fs::remove_file(&self.path).unwrap();
        std::fs::remove_dir(&self.directory).unwrap();
        world.resource_mut::<SaveLocation>().0 = self.original_location;
    }
}
