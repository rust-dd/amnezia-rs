//! Integration test for the audio converter: sound effects and music files are
//! copied into `output/audio/`, and unrelated files are ignored.

use std::fs;

#[test]
fn copies_sound_effects_and_music() {
    let base = std::env::temp_dir().join("amnezia_convert_audio_test");
    let _ = fs::remove_dir_all(&base);
    let input = base.join("in");
    fs::create_dir_all(input.join("Sound")).unwrap();
    fs::create_dir_all(input.join("Music")).unwrap();
    fs::write(input.join("Sound").join("Attack.wav"), b"RIFF").unwrap();
    fs::write(input.join("Sound").join("notes.txt"), b"ignore me").unwrap();
    fs::write(input.join("Music").join("Boss.mid"), b"MThd").unwrap();

    let output = base.join("out");
    let (effects, music) = amnezia_convert::convert_audio(&input, &output).unwrap();

    assert_eq!((effects, music), (1, 1));
    assert!(output.join("audio/Sound/Attack.wav").exists());
    assert!(output.join("audio/Music/Boss.mid").exists());
    assert!(!output.join("audio/Sound/notes.txt").exists());

    let _ = fs::remove_dir_all(&base);
}
