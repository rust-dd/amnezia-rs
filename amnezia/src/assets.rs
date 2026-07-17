//! Asset-path helpers shared across the game modules.

use std::path::Path;
use std::sync::OnceLock;

/// The converted-assets root, resolved once. Debug builds use the in-tree
/// `assets/`; release builds use `assets/` inside the app bundle (next to the
/// executable's `Contents/Resources`), so a distributed `.app` is self-contained.
pub fn asset_root() -> &'static str {
    static ROOT: OnceLock<String> = OnceLock::new();
    ROOT.get_or_init(|| {
        if cfg!(debug_assertions) {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../assets").to_string()
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|dir| dir.join("../Resources/assets")))
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "assets".to_string())
        }
    })
}

/// Load and deserialise a RON file, panicking with context on failure.
pub fn load_ron<T: serde::de::DeserializeOwned>(path: &str) -> T {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
    ron::from_str(&text).unwrap_or_else(|e| panic!("parsing {path}: {e}"))
}

/// Resolve a graphic name to its PNG path under `graphics/<subdir>/`, matching
/// the on-disk filename case-insensitively (RM2000 names differ in case).
pub fn resolve_png(subdir: &str, name: &str) -> String {
    let dir = format!("{}/graphics/{subdir}", asset_root());
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
