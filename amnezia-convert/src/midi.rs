//! Offline MIDI-to-Vorbis conversion; the game plays only the rendered OGG files.

use anyhow::{Context, Result};
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use std::io::Cursor;
use std::num::{NonZeroU8, NonZeroU32};
use std::path::Path;
use std::sync::Arc;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};

/// Untracked conversion SoundFont, resolved independently of the working directory.
const SOUNDFONT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/soundfont/GeneralUser-GS.sf2"
);

const SAMPLE_RATE: u32 = 44_100;

/// Small blocks avoid libvorbis's performance degradation on whole-track buffers.
const ENCODE_BLOCK_FRAMES: usize = 8_192;

/// Equivalent to `ffmpeg -q:a 4` on vorbis_rs's `-0.2..=1.0` scale.
const VORBIS_QUALITY: f32 = 0.4;

/// Render each `*.mid` to a sibling `.ogg`, overwriting existing files and returning
/// the encoded count. Missing SoundFonts and invalid tracks are logged and skipped.
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

fn synthesize_file(soundfont: &Arc<SoundFont>, midi: &Path, out: &Path) -> Result<()> {
    let bytes = std::fs::read(midi).with_context(|| format!("reading {}", midi.display()))?;
    synthesize_to_ogg(soundfont, &bytes, out)
}

fn synthesize_to_ogg(soundfont: &Arc<SoundFont>, midi_bytes: &[u8], out: &Path) -> Result<()> {
    let (left, right) = render_midi(soundfont, midi_bytes)?;
    encode_ogg(&left, &right, out)
}

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
mod tests;
