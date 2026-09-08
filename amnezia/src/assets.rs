//! Asset-path helpers shared across the game modules.

use std::collections::HashMap;
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
                .map(|exe| {
                    release_assets(
                        &exe,
                        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets")),
                    )
                })
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "assets".to_string())
        }
    })
}

fn release_assets(executable: &Path, workspace: &Path) -> std::path::PathBuf {
    let dir = executable.parent().unwrap_or_else(|| Path::new("."));
    [
        dir.join("../Resources/assets"),
        dir.join("assets"),
        workspace.to_path_buf(),
    ]
    .into_iter()
    .find(|path| path.join("hero.ron").is_file())
    .unwrap_or_else(|| dir.join("assets"))
}

/// Load and deserialise a RON file, panicking with context on failure.
pub fn load_ron<T: serde::de::DeserializeOwned>(path: &str) -> T {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
    ron::from_str(&text).unwrap_or_else(|e| panic!("parsing {path}: {e}"))
}

/// Resolve a graphic name to its PNG path under `graphics/<subdir>/`, matching
/// the on-disk filename case-insensitively (RM2000 names differ in case).
pub fn resolve_png(subdir: &str, name: &str) -> String {
    static GRAPHICS: OnceLock<HashMap<(String, String), String>> = OnceLock::new();
    let graphics = GRAPHICS.get_or_init(|| {
        let mut graphics = HashMap::new();
        if let Ok(categories) = std::fs::read_dir(Path::new(asset_root()).join("graphics")) {
            for category in categories.flatten() {
                let Ok(files) = std::fs::read_dir(category.path()) else {
                    continue;
                };
                let dir = category.file_name().to_string_lossy().into_owned();
                for file in files.flatten() {
                    let file = file.file_name().to_string_lossy().into_owned();
                    if let Some(stem) = file.to_lowercase().strip_suffix(".png") {
                        graphics
                            .entry((dir.to_lowercase(), stem.to_string()))
                            .or_insert_with(|| format!("graphics/{dir}/{file}"));
                    }
                }
            }
        }
        graphics
    });
    graphics
        .get(&(subdir.to_lowercase(), name.to_lowercase()))
        .cloned()
        .unwrap_or_else(|| format!("graphics/{subdir}/{name}.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charset_lookup_is_case_insensitive_and_uses_a_real_file() {
        let path = resolve_png("CharSet", "vehicle");
        assert_eq!(path, resolve_png("charset", "VEHICLE"));
        assert!(Path::new(asset_root()).join(path).is_file());
    }

    #[test]
    fn standalone_release_can_use_workspace_assets() {
        let workspace = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets"));
        assert_eq!(
            release_assets(Path::new("/nonexistent/amnezia/release/amnezia"), workspace),
            workspace
        );
    }
}
