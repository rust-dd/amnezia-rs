use super::*;

fn directory(files: &[&str]) -> Directory {
    Directory::new(files.iter().map(|file| (*file).to_owned()))
}

#[test]
fn lookup_preserves_filename_case_unicode_and_embedded_dots() {
    let files = directory(&["House.OGG", "ÁRVÍZTŰRŐ.Wav", "Door.v2.wav"]);
    assert_eq!(files.find("HOUSE", &["ogg", "wav"]), Some("House.OGG"));
    assert_eq!(files.find("árvíztűrő", &["wav"]), Some("ÁRVÍZTŰRŐ.Wav"));
    assert_eq!(files.find("door.V2", &["wav"]), Some("Door.v2.wav"));
}

#[test]
fn extension_preference_is_independent_of_directory_order() {
    for names in [["Field.WAV", "FIELD.ogg"], ["FIELD.ogg", "Field.WAV"]] {
        let files = directory(&names);
        assert_eq!(files.find("Field", &["ogg", "wav"]), Some("FIELD.ogg"));
        assert_eq!(files.find("Field", &["wav", "ogg"]), Some("Field.WAV"));
        assert_eq!(files.find("Field", &["mp3", "wav"]), Some("Field.WAV"));
    }
}

#[test]
fn case_collisions_keep_the_first_directory_entry() {
    let files = directory(&["Theme.WAV", "theme.wav", "THEME.wav"]);
    assert_eq!(files.find("theme", &["wav"]), Some("Theme.WAV"));
}

#[test]
fn missing_or_unsupported_names_remain_silent() {
    let files = directory(&["Theme.mid", "Door.wav", "door"]);
    for name in ["Theme", "missing", "(OFF)", "", "Door.wav", "../Door"] {
        assert_eq!(files.find(name, &["ogg", "wav"]), None, "{name}");
    }
    assert_eq!(files.find("Door", &[]), None);
    assert_eq!(files.find("Door", &["WAV"]), None);
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(tag: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "amnezia-audio-paths-{tag}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn paths(&self) -> AudioPaths {
        AudioPaths {
            root: self.0.clone(),
            directories: HashMap::new(),
        }
    }

    fn add(&self, category: &str, name: &str) {
        let directory = self.0.join(category);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(name), []).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn successful_directory_scan_is_reused_without_further_filesystem_reads() {
    let fixture = Fixture::new("cache");
    fixture.add("Music", "House.OGG");
    fixture.add("Sound", "HOUSE.wav");
    let mut paths = fixture.paths();
    assert_eq!(
        paths.resolve("Music", "house", &["ogg", "wav"]),
        Some("audio/Music/House.OGG".into())
    );
    std::fs::rename(fixture.0.join("Music"), fixture.0.join("loaded-music")).unwrap();
    for _ in 0..3 {
        assert_eq!(
            paths.resolve("Music", "HOUSE", &["ogg", "wav"]),
            Some("audio/Music/House.OGG".into())
        );
        assert_eq!(
            paths.resolve("Sound", "house", &["wav"]),
            Some("audio/Sound/HOUSE.wav".into())
        );
        assert_eq!(paths.resolve("Music", "missing", &["ogg", "wav"]), None);
    }
}

#[test]
fn a_failed_scan_does_not_poison_future_lookups() {
    let fixture = Fixture::new("retry");
    let mut paths = fixture.paths();
    assert_eq!(paths.resolve("Sound", "Door", &["wav"]), None);
    assert!(paths.directories.is_empty());
    fixture.add("Sound", "Door.wav");
    assert_eq!(
        paths.resolve("Sound", "Door", &["wav"]),
        Some("audio/Sound/Door.wav".into())
    );
}

#[test]
fn every_shipped_audio_path_matches_the_uncached_lookup() {
    let mut paths = AudioPaths::default();
    let mut checked = 0;
    for category in ["Music", "Sound"] {
        let files = std::fs::read_dir(paths.root.join(category))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        for file in &files {
            let Some((stem, extension)) = file.rsplit_once('.') else {
                continue;
            };
            if !matches!(extension.to_lowercase().as_str(), "ogg" | "wav") {
                continue;
            }
            checked += 1;
            for name in [stem.to_owned(), stem.to_uppercase(), stem.to_lowercase()] {
                for exts in [["ogg", "wav"], ["wav", "ogg"]] {
                    let lower = name.to_lowercase();
                    let expected = exts.iter().find_map(|ext| {
                        let target = format!("{lower}.{ext}");
                        files
                            .iter()
                            .find(|file| file.to_lowercase() == target)
                            .map(|file| format!("audio/{category}/{file}"))
                    });
                    assert_eq!(paths.resolve(category, &name, &exts), expected, "{name}");
                }
            }
        }
    }
    assert_eq!(checked, 151);
}
