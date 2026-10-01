//! Rasterize Hungarian text and UI symbols to preview `rmg2000.ttf` outside the game.
//!
//!     cargo run -p amnezia-convert --bin font_preview

use anyhow::{Result, anyhow};
use fontdue::{Font, FontSettings};
use image::{Rgba, RgbaImage};

fn main() -> Result<()> {
    let bytes = std::fs::read("amnezia/fonts/rmg2000.ttf")?;
    let font =
        Font::from_bytes(bytes.as_slice(), FontSettings::default()).map_err(|e| anyhow!("{e}"))?;
    let lines = [
        "Arvizturo tukorfurogep",
        "Árvíztűrő tükörfúrógép",
        "Harc  Auto  Menekülés",
        "▶ Támadás  Képesség  ×3  0123",
    ];
    let px = 24.0f32;
    let (w, h) = (820u32, 150u32);
    let mut img = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]));
    for (li, line) in lines.iter().enumerate() {
        let baseline = 28 + li as i32 * 34;
        let mut pen = 6i32;
        for ch in line.chars() {
            let (m, bmp) = font.rasterize(ch, px);
            for row in 0..m.height {
                for col in 0..m.width {
                    let cov = bmp[row * m.width + col];
                    if cov == 0 {
                        continue;
                    }
                    let x = pen + m.xmin + col as i32;
                    let y = baseline - m.ymin - m.height as i32 + row as i32;
                    if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
                        let v = 255 - cov;
                        img.put_pixel(x as u32, y as u32, Rgba([v, v, v, 255]));
                    }
                }
            }
            pen += m.advance_width.round() as i32;
        }
    }
    let out = format!(
        "{}/font_preview.png",
        std::env::var("CLAUDE_JOB_DIR").unwrap_or_else(|_| ".".into()) + "/tmp"
    );
    img.save(&out)?;
    println!("wrote {out}");
    Ok(())
}
