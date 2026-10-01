//! Conversion of the game's audio: copying sound effects and music sources and
//! synthesizing the MIDI tracks into decodable `.ogg` files.

use anyhow::{Context, Result};
use std::path::Path;

/// Copy WAV/MIDI sources into `output/audio/` and render MIDI to sibling OGG files.
/// Returns source-file counts `(sound_effects, music_tracks)`, excluding synthesized OGGs.
pub fn convert_audio(input: &Path, output: &Path) -> Result<(usize, usize)> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let audio = output.join("audio");
    let effects = copy_audio_dir(&input.join("Sound"), &audio.join("Sound"), "wav")?;
    let music_out = audio.join("Music");
    let music = copy_audio_dir(&input.join("Music"), &music_out, "mid")?
        + copy_audio_dir(&input.join("Music"), &music_out, "wav")?;
    crate::midi::synthesize_dir(&music_out);
    Ok((effects, music))
}

/// Copy every file with `extension` (case-insensitive) from `dir` into `out`,
/// returning the number copied. A missing `dir` copies nothing.
fn copy_audio_dir(dir: &Path, out: &Path, extension: &str) -> Result<usize> {
    if !dir.is_dir() {
        return Ok(0);
    }
    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    let mut count = 0;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let matches = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case(extension));
        if !matches {
            continue;
        }
        let name = path.file_name().context("audio file has no name")?;
        std::fs::copy(&path, out.join(name))
            .with_context(|| format!("copying {}", path.display()))?;
        count += 1;
    }
    Ok(count)
}
