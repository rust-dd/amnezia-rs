use std::path::Path;

const DATABASES: [&str; 19] = [
    "actors.ron",
    "animations.ron",
    "attributes.ron",
    "chipsets.ron",
    "common_events.ron",
    "hero.ron",
    "items.ron",
    "map_info.ron",
    "monsters.ron",
    "skills.ron",
    "start.ron",
    "states.ron",
    "system.ron",
    "terms.ron",
    "terrains.ron",
    "troops.ron",
    "vehicles.ron",
    "i18n/hu.ron",
    "i18n/en.ron",
];

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    ron::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

fn inspect(root: &Path) -> Result<usize, String> {
    for name in DATABASES {
        read::<serde::de::IgnoredAny>(&root.join(name))?;
    }
    read::<amnezia_data::Hero>(&root.join("hero.ron"))?;
    let start = read::<amnezia_data::Start>(&root.join("start.ron"))?;
    let mut maps = 0;
    let mut start_found = false;
    for entry in std::fs::read_dir(root.join("maps")).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().is_none_or(|extension| extension != "ron") {
            continue;
        }
        let map = read::<amnezia_data::Map>(&path)?;
        if map.lower.len() != (map.width * map.height) as usize
            || map.upper.len() != map.lower.len()
        {
            return Err(format!("{}: invalid map dimensions", path.display()));
        }
        if path
            .file_name()
            .is_some_and(|name| name == format!("map_{:04}.ron", start.map_id).as_str())
        {
            if start.x >= map.width || start.y >= map.height {
                return Err("new-game position is outside its map".into());
            }
            start_found = true;
        }
        maps += 1;
    }
    if !start_found {
        return Err("new-game map is missing".into());
    }
    for category in [
        "graphics/CharSet",
        "graphics/ChipSet",
        "audio/Music",
        "audio/Sound",
    ] {
        if !root.join(category).is_dir() {
            return Err(format!("missing asset directory: {category}"));
        }
    }
    Ok(maps)
}

pub(crate) fn check() -> Result<(), String> {
    let root = Path::new(super::asset_root())
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let maps = inspect(&root)?;
    let slot = crate::save::SaveLocation::default().0;
    let saves = slot.parent().ok_or("save path has no parent")?;
    if !saves.is_absolute() {
        return Err("save directory must be absolute".into());
    }
    println!(
        "Amnezia {} ({}/{})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!("Assets: {}", root.display());
    println!("Saves: {}", saves.display());
    if std::env::args().any(|argument| argument == "--check-save-permissions") {
        check_writable(saves).map_err(|error| format!("{}: {error}", saves.display()))?;
        println!("Save permissions verified with a temporary write/rename/read; probe removed.");
    }
    println!(
        "Installation data verified: {} databases/catalogs and {maps} maps; no saves written.",
        DATABASES.len()
    );
    Ok(())
}

fn check_writable(directory: &Path) -> std::io::Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(directory)?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let probe = directory.join(format!(
        ".amnezia-installation-check-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&probe)?;
    let pending = probe.join("pending");
    let completed = probe.join("completed");
    let mut file = std::fs::File::create_new(&pending)?;
    file.write_all(b"Amnezia save permission check")?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&pending, &completed)?;
    if std::fs::read(&completed)? != b"Amnezia save permission check" {
        return Err(std::io::Error::other(
            "save permission probe contents changed",
        ));
    }
    std::fs::remove_file(completed)?;
    std::fs::remove_dir(probe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_permission_probe_does_not_touch_existing_slots() {
        let root =
            std::env::temp_dir().join(format!("amnezia-installation-test-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let slot = root.join("slot1.ron");
        std::fs::write(&slot, b"existing player save").unwrap();
        check_writable(&root).unwrap();
        assert_eq!(std::fs::read(&slot).unwrap(), b"existing player save");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_file(slot).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn shipped_installation_contains_all_campaign_maps() {
        assert_eq!(inspect(Path::new(super::super::asset_root())).unwrap(), 276);
    }

    #[test]
    fn missing_installation_reports_the_required_file_without_creating_it() {
        let root = std::env::temp_dir().join(format!(
            "amnezia-missing-installation-{}",
            std::process::id()
        ));
        assert!(!root.exists());
        assert!(inspect(&root).unwrap_err().contains("actors.ron"));
        assert!(!root.exists());
    }
}
