use anyhow::{Context, Result, ensure};
use std::io::Cursor;
use std::path::Path;

fn vorbis_frames(bytes: &[u8]) -> Result<(u32, u64)> {
    let mut offset = 0;
    let mut rate = None;
    let mut frames = 0;
    while offset < bytes.len() {
        let header = bytes
            .get(offset..offset + 27)
            .context("truncated Ogg header")?;
        ensure!(&header[..4] == b"OggS", "invalid Ogg page signature");
        let granule = u64::from_le_bytes(header[6..14].try_into().unwrap());
        if granule != u64::MAX {
            frames = frames.max(granule);
        }
        let segments = header[26] as usize;
        let laces = bytes
            .get(offset + 27..offset + 27 + segments)
            .context("truncated Ogg lacing")?;
        let start = offset + 27 + segments;
        let end = start + laces.iter().map(|&n| n as usize).sum::<usize>();
        let payload = bytes.get(start..end).context("truncated Ogg payload")?;
        if offset == 0 {
            ensure!(
                payload.len() >= 16 && &payload[..7] == b"\x01vorbis",
                "missing Vorbis identification"
            );
            ensure!(payload[11] == 2, "converted MIDI must be stereo");
            rate = Some(u32::from_le_bytes(payload[12..16].try_into().unwrap()));
        }
        offset = end;
    }
    Ok((rate.context("empty Ogg stream")?, frames))
}

fn audit(source: &Path, converted: &Path) -> Result<()> {
    let mut midi_count = 0;
    let mut wave_count = 0;
    for category in ["Music", "Sound"] {
        let mut paths = std::fs::read_dir(source.join(category))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        paths.sort();
        for path in paths {
            let extension = path
                .extension()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_lowercase();
            if !matches!(extension.as_str(), "wav" | "mid") {
                continue;
            }
            let name = path.file_name().unwrap();
            let copy = converted.join("audio").join(category).join(name);
            let bytes = std::fs::read(&path)?;
            ensure!(
                bytes
                    == std::fs::read(&copy)
                        .with_context(|| format!("reading {}", copy.display()))?,
                "{} differs from the original bytes",
                copy.display()
            );
            if extension == "wav" {
                wave_count += 1;
                continue;
            }
            let midi = rustysynth::MidiFile::new(&mut Cursor::new(bytes))
                .context("parsing original MIDI")?;
            let ogg = copy.with_extension("ogg");
            let (rate, actual) = vorbis_frames(&std::fs::read(&ogg)?)?;
            ensure!(
                rate == 44_100,
                "{} has unexpected sample rate {rate}",
                ogg.display()
            );
            let expected = (midi.get_length() * f64::from(rate)).ceil().max(1.0) as u64;
            ensure!(
                actual == expected,
                "{}: expected {expected} loop frames from MIDI end-of-track, got {actual}",
                ogg.display()
            );
            println!(
                "{}: {actual} frames at {rate} Hz, original MIDI loop duration retained",
                name.to_string_lossy()
            );
            midi_count += 1;
        }
    }
    println!(
        "Verified {midi_count} MIDI/OGG loop lengths and {wave_count} byte-identical original WAV files. Soundfont timbre is not original-runtime parity."
    );
    Ok(())
}

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    audit(&root.join("original"), &root.join("assets"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_or_truncated_audio_cannot_report_a_valid_duration() {
        for bytes in [&b""[..], &b"OggS"[..], &[0; 27][..]] {
            assert!(vorbis_frames(bytes).is_err());
        }
    }
}
