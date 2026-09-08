use super::SaveGame;
use bevy::prelude::error;
use ron::ser::PrettyConfig;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

const SLOT_FILE: &str = "slot1.ron";

pub(super) fn save_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if cfg!(debug_assertions) {
            return PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../saves"));
        }
        let home_dir = std::env::var_os("HOME").map(PathBuf::from);
        let app_data = std::env::var_os(if cfg!(target_os = "windows") {
            "LOCALAPPDATA"
        } else {
            "XDG_DATA_HOME"
        })
        .map(PathBuf::from);
        user_save_dir(
            std::env::consts::OS,
            home_dir.as_deref(),
            app_data.as_deref(),
        )
        .or_else(|| executable_save_dir().ok())
        .expect("cannot determine a persistent save directory")
    })
}

fn user_save_dir(os: &str, home_dir: Option<&Path>, app_data: Option<&Path>) -> Option<PathBuf> {
    let home_dir = home_dir.filter(|p| p.is_absolute());
    let app_data = app_data.filter(|p| p.is_absolute());
    match os {
        "macos" => home_dir.map(|p| p.join("Library/Application Support/Amnezia/saves")),
        "windows" => app_data.map(|p| p.join("Amnezia/saves")),
        _ => app_data
            .map(Path::to_path_buf)
            .or_else(|| home_dir.map(|p| p.join(".local/share")))
            .map(|p| p.join("amnezia/saves")),
    }
}

fn executable_save_dir() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    Ok(exe
        .parent()
        .expect("executable has a directory")
        .join("saves"))
}

pub(super) fn save_path() -> PathBuf {
    save_dir().join(SLOT_FILE)
}

/// Old app bundles stored saves beside their executable; read them without moving them.
pub(super) fn legacy_save_path() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        return None;
    }
    executable_save_dir().ok().map(|p| p.join(SLOT_FILE))
}

pub(super) fn slot_exists(path: &Path) -> bool {
    path.is_file()
}

pub(super) fn write_save(path: &Path, game: &SaveGame) -> Result<(), String> {
    let ron =
        ron::ser::to_string_pretty(game, PrettyConfig::default()).map_err(|e| e.to_string())?;
    atomic_write(path, ron.as_bytes()).map_err(|e| e.to_string())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let temp = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

pub(super) fn read_save(path: &Path) -> Option<SaveGame> {
    let legacy = (path == save_path() && !path.exists())
        .then(legacy_save_path)
        .flatten();
    let source = legacy.as_deref().filter(|p| p.is_file()).unwrap_or(path);
    let text = std::fs::read_to_string(source).ok()?;
    match ron::from_str(&text) {
        Ok(game) => Some(game),
        Err(e) => {
            error!("load failed: parsing {}: {e}", source.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_saves_live_outside_the_application_bundle() {
        let home_dir = Path::new("/Users/player");
        assert_eq!(
            user_save_dir("macos", Some(home_dir), None).unwrap(),
            home_dir.join("Library/Application Support/Amnezia/saves"),
        );
        assert_eq!(
            user_save_dir("linux", Some(home_dir), Some(Path::new("/data"))).unwrap(),
            Path::new("/data/amnezia/saves"),
        );
        assert!(user_save_dir("macos", Some(Path::new("relative")), None).is_none());
    }

    #[test]
    fn successful_save_replaces_the_whole_slot() {
        let path = super::super::tests::temp_slot("atomic");
        std::fs::write(&path, "old save").unwrap();
        atomic_write(&path, b"new save").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new save");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn failed_replacement_preserves_the_existing_destination() {
        let path = super::super::tests::temp_slot("atomic_failure");
        std::fs::create_dir_all(&path).unwrap();
        let existing = path.join("existing.ron");
        std::fs::write(&existing, "keep me").unwrap();
        assert!(atomic_write(&path, b"new save").is_err());
        assert_eq!(std::fs::read_to_string(&existing).unwrap(), "keep me");
        std::fs::remove_file(existing).unwrap();
        std::fs::remove_dir(path).unwrap();
    }
}
