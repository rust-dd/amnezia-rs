use super::*;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

mod soundfont;

struct Output(PathBuf);

impl Drop for Output {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn push_vlq(buffer: &mut Vec<u8>, value: u32) {
    let mut bytes = vec![(value & 0x7f) as u8];
    let mut rest = value >> 7;
    while rest > 0 {
        bytes.push(((rest & 0x7f) as u8) | 0x80);
        rest >>= 7;
    }
    bytes.reverse();
    buffer.extend_from_slice(&bytes);
}

/// Two quarter notes at 480 ticks per quarter: one second at the default tempo.
fn tiny_midi() -> Vec<u8> {
    let mut track = Vec::new();
    for note in [60_u8, 64] {
        push_vlq(&mut track, 0);
        track.extend_from_slice(&[0x90, note, 0x64]);
        push_vlq(&mut track, 480);
        track.extend_from_slice(&[0x80, note, 0x00]);
    }
    track.extend_from_slice(&[0x00, 0xff, 0x2f, 0x00]);

    let mut midi = Vec::new();
    midi.extend_from_slice(b"MThd");
    midi.extend_from_slice(&6_u32.to_be_bytes());
    midi.extend_from_slice(&0_u16.to_be_bytes());
    midi.extend_from_slice(&1_u16.to_be_bytes());
    midi.extend_from_slice(&480_u16.to_be_bytes());
    midi.extend_from_slice(b"MTrk");
    midi.extend_from_slice(&(track.len() as u32).to_be_bytes());
    midi.extend_from_slice(&track);
    midi
}

fn check_synthesis(soundfont: &Arc<SoundFont>) {
    let midi = tiny_midi();
    let (left, right) = render_midi(soundfont, &midi).unwrap();
    assert_eq!(left.len(), SAMPLE_RATE as usize);
    assert_eq!(right.len(), left.len());
    for samples in [&left, &right] {
        assert!(samples.iter().all(|value| value.is_finite()));
        assert!(samples.iter().any(|value| value.abs() > 0.001));
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let out = Output(
        std::env::temp_dir().join(format!("amnezia-midi-{}-{stamp}.ogg", std::process::id())),
    );
    synthesize_to_ogg(soundfont, &midi, &out.0).unwrap();
    let bytes = std::fs::read(&out.0).unwrap();
    assert!(bytes.starts_with(b"OggS"));
    assert!(bytes.len() > 100);
}

#[test]
fn synthesizes_a_non_empty_ogg() {
    check_synthesis(&soundfont::fixture());
}

#[test]
#[ignore = "requires the untracked GeneralUser-GS.sf2 conversion SoundFont"]
fn synthesizes_with_conversion_soundfont() {
    check_synthesis(&load_soundfont().expect("conversion SoundFont loads"));
}
