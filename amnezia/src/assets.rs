//! Asset-path helpers shared across the game modules.

use std::path::Path;

/// Absolute path to the converted assets, resolved at compile time so the game
/// runs regardless of the current working directory.
pub const ASSET_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

/// Load and deserialise a RON file, panicking with context on failure.
pub fn load_ron<T: serde::de::DeserializeOwned>(path: &str) -> T {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
    ron::from_str(&text).unwrap_or_else(|e| panic!("parsing {path}: {e}"))
}

/// Resolve a graphic name to its PNG path under `graphics/<subdir>/`, matching
/// the on-disk filename case-insensitively (RM2000 names differ in case).
pub fn resolve_png(subdir: &str, name: &str) -> String {
    let dir = format!("{ASSET_ROOT}/graphics/{subdir}");
    let target = format!("{}.png", name.to_lowercase());
    if let Ok(entries) = std::fs::read_dir(Path::new(&dir)) {
        for entry in entries.flatten() {
            let file = entry.file_name();
            let file = file.to_string_lossy();
            if file.to_lowercase() == target {
                return format!("graphics/{subdir}/{file}");
            }
        }
    }
    format!("graphics/{subdir}/{name}.png")
}
