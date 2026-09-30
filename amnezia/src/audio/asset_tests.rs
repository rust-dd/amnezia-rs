use bevy::audio::{AudioSource, Decodable};
use std::path::Path;

#[test]
fn every_shipped_music_and_sound_file_decodes_completely_without_audio_output() {
    let mut files = 0;
    let mut total_samples = 0_u64;
    for category in ["Music", "Sound"] {
        let root = Path::new(crate::assets::asset_root())
            .join("audio")
            .join(category);
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            let extension = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if !matches!(extension.as_str(), "ogg" | "wav") {
                continue;
            }
            let source = AudioSource {
                bytes: std::fs::read(&path).unwrap().into(),
            };
            let mut samples = 0_u64;
            for sample in source.decoder() {
                assert!(sample.is_finite(), "{}: non-finite sample", path.display());
                samples += 1;
            }
            assert!(samples > 0, "{}: empty audio", path.display());
            total_samples += samples;
            files += 1;
        }
    }
    assert_eq!(files, 151);
    println!(
        "Decoded {files} original/converted audio files, {total_samples} samples, without opening an output device"
    );
}
