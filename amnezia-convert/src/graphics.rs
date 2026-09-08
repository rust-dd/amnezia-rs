//! Conversion of XYZ graphics and preservation of the original PNG assets.

use anyhow::{Context, Result};
use std::path::Path;

const GRAPHIC_CATEGORIES: &[&str] = &[
    "Backdrop", "Battle", "CharSet", "ChipSet", "FaceSet", "GameOver", "Monster", "Panorama",
    "Picture", "System", "Title",
];

const TRANSPARENT_CATEGORIES: &[&str] = &[
    "Battle", "CharSet", "ChipSet", "Monster", "Picture", "System",
];

/// Convert every XYZ or PNG graphic under `input`'s category directories into a
/// PNG under `output/graphics/<Category>/`, returning the number written.
/// Categories in `TRANSPARENT_CATEGORIES` map palette index 0 to a
/// transparent alpha; the rest stay fully opaque.
pub fn convert_graphics(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let mut count = 0;
    for category in GRAPHIC_CATEGORIES {
        let dir = input.join(category);
        if !dir.is_dir() {
            continue;
        }
        let transparent = TRANSPARENT_CATEGORIES.contains(category);
        let out_dir = output.join("graphics").join(category);
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("png"))
            {
                std::fs::create_dir_all(&out_dir)?;
                std::fs::copy(
                    &path,
                    out_dir.join(path.file_name().context("graphic has no filename")?),
                )?;
                count += 1;
                continue;
            }
            let is_xyz = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("xyz"));
            if !is_xyz {
                continue;
            }
            let bytes =
                std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            let decoded = xyz::decode(&bytes, transparent)
                .with_context(|| format!("decoding {}", path.display()))?;
            let buffer = image::RgbaImage::from_raw(
                decoded.width as u32,
                decoded.height as u32,
                decoded.rgba,
            )
            .context("decoded RGBA buffer has an unexpected size")?;
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out = out_dir.join(format!("{stem}.png"));
            std::fs::create_dir_all(&out_dir)
                .with_context(|| format!("creating {}", out_dir.display()))?;
            buffer
                .save(&out)
                .with_context(|| format!("writing {}", out.display()))?;
            count += 1;
        }
    }
    Ok(count)
}
