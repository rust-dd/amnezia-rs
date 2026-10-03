use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Packaged audio directories do not change during a game session.
pub(super) struct AudioPaths {
    root: PathBuf,
    directories: HashMap<String, Directory>,
}

impl Default for AudioPaths {
    fn default() -> Self {
        Self {
            root: Path::new(crate::assets::asset_root()).join("audio"),
            directories: HashMap::new(),
        }
    }
}

impl AudioPaths {
    pub(super) fn resolve(&mut self, subdir: &str, name: &str, exts: &[&str]) -> Option<String> {
        if !self.directories.contains_key(subdir) {
            let files = std::fs::read_dir(self.root.join(subdir)).ok()?;
            let directory = Directory::new(
                files
                    .flatten()
                    .map(|entry| entry.file_name().to_string_lossy().into_owned()),
            );
            self.directories.insert(subdir.to_owned(), directory);
        }
        let file = self.directories.get(subdir)?.find(name, exts)?;
        Some(format!("audio/{subdir}/{file}"))
    }
}

struct Directory(HashMap<String, String>);

impl Directory {
    fn new(files: impl IntoIterator<Item = String>) -> Self {
        let mut index = HashMap::new();
        for file in files {
            index.entry(file.to_lowercase()).or_insert(file);
        }
        Self(index)
    }

    fn find(&self, name: &str, exts: &[&str]) -> Option<&str> {
        let lower = name.to_lowercase();
        exts.iter()
            .find_map(|ext| self.0.get(&format!("{lower}.{ext}")).map(String::as_str))
    }
}

#[cfg(test)]
mod tests;
