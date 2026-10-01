use std::io::{self, Write};
use std::path::Path;

pub(super) fn next_id(directory: &Path) -> u64 {
    #[derive(serde::Deserialize)]
    struct Latest {
        id: u64,
    }
    let latest = std::fs::read_to_string(directory.join("state.ron"))
        .ok()
        .and_then(|text| ron::from_str::<Latest>(&text).ok())
        .map(|state| state.id);
    let history = directory
        .read_dir()
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()?
                .strip_prefix("state-")?
                .strip_suffix(".ron")?
                .parse::<u64>()
                .ok()
        });
    history
        .chain(latest)
        .max()
        .map_or(0, |id| id.saturating_add(1))
}

pub(super) fn snapshot(directory: &Path, id: u64, text: &str, capture: bool) -> io::Result<()> {
    let temporary = directory.join("state.tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(temporary, directory.join("state.ron"))?;
    if capture {
        std::fs::write(directory.join(format!("state-{id:06}.ron")), text)?;
    }
    Ok(())
}

pub(super) fn append(path: &Path, text: &str) -> io::Result<()> {
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(log, "{text}")
}

pub(super) fn report(result: io::Result<()>) {
    if let Err(error) = result {
        bevy::log::warn!("playtest diagnostic write failed: {error}");
    }
}
