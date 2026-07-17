//! Offline MIDI synthesis for background music. Each RM2000 `.mid` track is
//! rendered to PCM with the bundled General MIDI soundfont (rustysynth) and
//! encoded to OGG Vorbis (vorbis_rs). This runs only at conversion time; the
//! shipped game plays the resulting `.ogg` and never sees a synthesizer.

use anyhow::{Context, Result};
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use std::io::Cursor;
use std::num::{NonZeroU8, NonZeroU32};
use std::path::Path;
use std::sync::Arc;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};

/// The bundled GeneralUser GS soundfont, resolved at compile time relative to
/// this crate so synthesis works regardless of the process working directory.
const SOUNDFONT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/soundfont/GeneralUser-GS.sf2"
);

/// Render sample rate: 44.1 kHz stereo, the rate the game's audio pipeline uses.
const SAMPLE_RATE: u32 = 44_100;

/// PCM frames per Vorbis encode block. libvorbis degrades sharply on very large
/// blocks, so whole-track PCM is fed in windows of roughly a fifth of a second.
const ENCODE_BLOCK_FRAMES: usize = 8_192;

/// VBR quality factor (vorbis_rs accepts `-0.2..=1.0`); `0.4` mirrors the
/// intended `ffmpeg -q:a 4` balance of size against fidelity.
const VORBIS_QUALITY: f32 = 0.4;

/// Render every `*.mid` directly under `music_dir` to a sibling `<stem>.ogg`,
/// returning the number encoded. Resilient by design: a missing soundfont or an
/// unreadable/corrupt track is logged and skipped so one bad file never aborts
/// the whole conversion. Existing `.ogg` files are overwritten.
pub fn synthesize_dir(music_dir: &Path) -> usize {
    if !music_dir.is_dir() {
        return 0;
    }
    let soundfont = match load_soundfont() {
        Ok(soundfont) => soundfont,
        Err(error) => {
            eprintln!("skipping BGM synthesis, no soundfont: {error:#}");
            return 0;
        }
    };
    let entries = match std::fs::read_dir(music_dir) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!(
                "skipping BGM synthesis, cannot read {}: {error:#}",
                music_dir.display()
            );
            return 0;
        }
    };
    let mut count = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_mid = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mid"));
        if !is_mid {
            continue;
        }
        let out = path.with_extension("ogg");
        match synthesize_file(&soundfont, &path, &out) {
            Ok(()) => count += 1,
            Err(error) => eprintln!("skipping BGM {}: {error:#}", path.display()),
        }
    }
    count
}

/// Read one MIDI file and encode it to `out`.
fn synthesize_file(soundfont: &Arc<SoundFont>, midi: &Path, out: &Path) -> Result<()> {
    let bytes = std::fs::read(midi).with_context(|| format!("reading {}", midi.display()))?;
    synthesize_to_ogg(soundfont, &bytes, out)
}

/// Synthesize `midi_bytes` with `soundfont` and encode the result to an OGG
/// Vorbis file at `out`.
fn synthesize_to_ogg(soundfont: &Arc<SoundFont>, midi_bytes: &[u8], out: &Path) -> Result<()> {
    let (left, right) = render_midi(soundfont, midi_bytes)?;
    encode_ogg(&left, &right, out)
}

/// Load the bundled General MIDI soundfont, shared across every track.
fn load_soundfont() -> Result<Arc<SoundFont>> {
    let mut file = std::fs::File::open(SOUNDFONT_PATH)
        .with_context(|| format!("opening soundfont {SOUNDFONT_PATH}"))?;
    let soundfont = SoundFont::new(&mut file).context("parsing soundfont")?;
    Ok(Arc::new(soundfont))
}

/// Render `midi_bytes` to planar stereo `f32` PCM at [`SAMPLE_RATE`]. The buffer
/// spans exactly the track's length so a seamless loop matches RM2000, which
/// restarts the MIDI from the top rather than letting the tail ring out.
fn render_midi(soundfont: &Arc<SoundFont>, midi_bytes: &[u8]) -> Result<(Vec<f32>, Vec<f32>)> {
    let mut cursor = Cursor::new(midi_bytes);
    let midi = Arc::new(MidiFile::new(&mut cursor).context("parsing MIDI")?);

    let settings = SynthesizerSettings::new(SAMPLE_RATE as i32);
    let synthesizer = Synthesizer::new(soundfont, &settings).context("creating synthesizer")?;
    let mut sequencer = MidiFileSequencer::new(synthesizer);
    sequencer.play(&midi, false);

    let frames = (midi.get_length() * f64::from(SAMPLE_RATE)).ceil().max(1.0) as usize;
    let mut left = vec![0.0_f32; frames];
    let mut right = vec![0.0_f32; frames];
    sequencer.render(&mut left, &mut right);
    Ok((left, right))
}

/// Encode planar stereo PCM to an OGG Vorbis file at `out`.
fn encode_ogg(left: &[f32], right: &[f32], out: &Path) -> Result<()> {
    let file = std::fs::File::create(out).with_context(|| format!("creating {}", out.display()))?;
    let sample_rate = NonZeroU32::new(SAMPLE_RATE).expect("sample rate is non-zero");
    let channels = NonZeroU8::new(2).expect("channel count is non-zero");
    let mut builder =
        VorbisEncoderBuilder::new(sample_rate, channels, std::io::BufWriter::new(file))
            .context("initialising the Vorbis encoder")?;
    builder.bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
        target_quality: VORBIS_QUALITY,
    });
    let mut encoder = builder.build().context("building the Vorbis encoder")?;

    for (left_block, right_block) in left
        .chunks(ENCODE_BLOCK_FRAMES)
        .zip(right.chunks(ENCODE_BLOCK_FRAMES))
    {
        encoder
            .encode_audio_block([left_block, right_block])
            .context("encoding an audio block")?;
    }
    encoder.finish().context("finalising the Vorbis stream")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Append `value` as a MIDI variable-length quantity.
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

    /// Build a minimal format-0 Standard MIDI File: two quarter notes at 480
    /// ticks per quarter, giving a track roughly one second long.
    fn tiny_midi() -> Vec<u8> {
        let mut track = Vec::new();
        for note in [60_u8, 64] {
            push_vlq(&mut track, 0);
            track.extend_from_slice(&[0x90, note, 0x64]);
            push_vlq(&mut track, 480);
            track.extend_from_slice(&[0x80, note, 0x00]);
        }
        push_vlq(&mut track, 0);
        track.extend_from_slice(&[0xff, 0x2f, 0x00]);

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

    #[test]
    fn synthesizes_a_non_empty_ogg() {
        let out = std::env::temp_dir().join("amnezia_convert_midi_test.ogg");
        let _ = std::fs::remove_file(&out);

        let soundfont = load_soundfont().expect("bundled soundfont loads");
        synthesize_to_ogg(&soundfont, &tiny_midi(), &out).expect("synthesis and encoding succeed");

        let bytes = std::fs::read(&out).expect("output ogg is readable");
        assert!(!bytes.is_empty(), "encoded ogg must not be empty");
        assert_eq!(&bytes[..4], b"OggS", "output must be a real Ogg stream");

        let _ = std::fs::remove_file(&out);
    }
}
