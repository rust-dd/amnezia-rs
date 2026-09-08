//! Convert EasyRPG's Western RM2000-compatible (ttyp0) and RMG2000-compatible
//! bitmap faces into TrueType outlines without changing their pixel grids.
//!
//! Run `cargo run -p amnezia-convert --bin build_font` from the workspace root.
//! The generated fonts are committed; normal builds need no reference checkout.

use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow};
use kurbo::BezPath;
use write_fonts::{
    FontBuilder,
    tables::{
        cmap::Cmap,
        glyf::{GlyfLocaBuilder, Glyph, SimpleGlyph},
        head::Head,
        hhea::Hhea,
        hmtx::{Hmtx, LongMetric},
        loca::LocaFormat,
        maxp::Maxp,
        name::{Name, NameRecord},
        os2::Os2,
        post::Post,
    },
    types::{FWord, GlyphId, NameId, UfWord, Version16Dot16},
};

const REF: &str = "reference/easyrpg-player/src/generated";
/// Font units per source pixel.
const PX: i32 = 64;
/// The em: the 12-row cell fills it.
const EM: u16 = 768;
/// Baseline at the bottom of row 9 — rows 10-11 become descenders.
const BASE: i32 = 128;

/// Glyphs RM2000 draws graphically (System.png), not as font characters, but our
/// chrome renders as text: the menu cursor, scroll/continue arrows, and the em
/// dash. All half-width. `(codepoint, 12 rows)`.
const DRAWN: &[(u32, [u16; 12])] = &[
    (9654, [0, 0, 0, 2, 6, 14, 30, 14, 6, 2, 0, 0]), // ▶
    (9664, [0, 0, 0, 16, 24, 28, 30, 28, 24, 16, 0, 0]), // ◀
    (9650, [0, 0, 0, 0, 12, 30, 63, 0, 0, 0, 0, 0]), // ▲
    (9660, [0, 0, 0, 0, 0, 63, 30, 12, 0, 0, 0, 0]), // ▼
    (8212, [0, 0, 0, 0, 0, 0, 63, 0, 0, 0, 0, 0]),   // — em dash
];
/// RMG2000 lacks the Hungarian double-acute letters: Ő ő Ű ű.
const TTYP0_FILL: &[u32] = &[336, 337, 368, 369];

type Bitmap = [u16; 12];

fn main() -> Result<()> {
    let mut rmg2000 = parse_header(&format!("{REF}/bitmapfont_rmg2000.h"))?;
    let ttyp0 = parse_header(&format!("{REF}/bitmapfont_ttyp0.h"))?;
    for &code in TTYP0_FILL {
        if let Some(&glyph) = ttyp0.get(&code) {
            rmg2000.entry(code).or_insert(glyph);
        }
    }
    for (file, family, mut glyphs) in [
        ("rm2000", "Amnezia RM2000", ttyp0),
        ("rmg2000", "Amnezia RMG2000", rmg2000),
    ] {
        add_symbols(&mut glyphs);
        let bytes = build_font(&glyphs, family)?;
        let path = format!("amnezia/fonts/{file}.ttf");
        std::fs::write(&path, &bytes).context("write ttf")?;
        println!(
            "wrote {path} ({} glyphs, {} bytes)",
            glyphs.len(),
            bytes.len()
        );
    }
    Ok(())
}

fn add_symbols(glyphs: &mut BTreeMap<u32, (bool, Bitmap)>) {
    for &(code, rows) in DRAWN {
        glyphs.insert(code, (false, rows));
    }
    glyphs.entry(32).or_insert((false, [0; 12]));
}

fn build_font(glyphs: &BTreeMap<u32, (bool, Bitmap)>, family: &str) -> Result<Vec<u8>> {
    let mut builder = GlyfLocaBuilder::new();
    let mut mappings = Vec::new();
    let mut metrics = vec![LongMetric::new((6 * PX) as u16, 0)];
    builder
        .add_glyph(&Glyph::Simple(SimpleGlyph::default()))
        .map_err(|e| anyhow!("notdef: {e}"))?;

    let mut gid = 1u16;
    for (&code, &(full, rows)) in glyphs {
        if code < 32 {
            continue;
        }
        let path = glyph_path(full, &rows);
        let glyph = if path.elements().is_empty() {
            SimpleGlyph::default()
        } else {
            SimpleGlyph::from_bezpath(&path).map_err(|e| anyhow!("glyph U+{code:04X}: {e:?}"))?
        };
        builder
            .add_glyph(&Glyph::Simple(glyph))
            .map_err(|e| anyhow!("add U+{code:04X}: {e}"))?;
        let left = rows
            .iter()
            .filter(|&&r| r != 0)
            .map(|r| r.trailing_zeros())
            .min()
            .unwrap_or(0);
        metrics.push(LongMetric::new(
            (if full { 12 } else { 6 } * PX) as u16,
            (left as i32 * PX) as i16,
        ));
        if let Some(ch) = char::from_u32(code) {
            mappings.push((ch, GlyphId::new(u32::from(gid))));
        }
        gid += 1;
    }

    let (glyf, loca, loca_format) = builder.build();
    let num_glyphs = gid;
    let num_h_metrics = metrics.len() as u16;
    let ascent = (EM as i32 - BASE) as i16;
    let descent = -(BASE as i16);

    let head = Head {
        units_per_em: EM,
        x_min: 0,
        y_min: descent,
        x_max: EM as i16,
        y_max: ascent,
        index_to_loc_format: match loca_format {
            LocaFormat::Short => 0,
            LocaFormat::Long => 1,
        },
        ..Default::default()
    };
    let hhea = Hhea {
        ascender: FWord::new(ascent),
        descender: FWord::new(descent),
        line_gap: FWord::new(0),
        advance_width_max: UfWord::new(EM),
        min_left_side_bearing: FWord::new(0),
        min_right_side_bearing: FWord::new(0),
        x_max_extent: FWord::new(EM as i16),
        caret_slope_rise: 1,
        caret_slope_run: 0,
        caret_offset: 0,
        number_of_h_metrics: num_h_metrics,
    };
    let maxp = Maxp {
        num_glyphs,
        ..Default::default()
    };
    let post = Post {
        version: Version16Dot16::VERSION_3_0,
        ..Default::default()
    };
    let cmap = Cmap::from_mappings(mappings).map_err(|e| anyhow!("cmap: {e:?}"))?;
    let hmtx = Hmtx::new(metrics, Vec::new());
    let name = Name::new(vec![
        name_record(NameId::FAMILY_NAME, family),
        name_record(NameId::SUBFAMILY_NAME, "Regular"),
        name_record(NameId::FULL_NAME, family),
        name_record(NameId::POSTSCRIPT_NAME, &family.replace(' ', "-")),
    ]);
    let os2 = Os2 {
        x_avg_char_width: (6 * PX) as i16,
        s_typo_ascender: ascent,
        s_typo_descender: descent,
        us_win_ascent: ascent as u16,
        us_win_descent: (-descent) as u16,
        ..Default::default()
    };

    let mut fb = FontBuilder::new();
    fb.add_table(&head).map_err(|e| anyhow!("head: {e}"))?;
    fb.add_table(&hhea).map_err(|e| anyhow!("hhea: {e}"))?;
    fb.add_table(&maxp).map_err(|e| anyhow!("maxp: {e}"))?;
    fb.add_table(&hmtx).map_err(|e| anyhow!("hmtx: {e}"))?;
    fb.add_table(&cmap).map_err(|e| anyhow!("cmap: {e}"))?;
    fb.add_table(&glyf).map_err(|e| anyhow!("glyf: {e}"))?;
    fb.add_table(&loca).map_err(|e| anyhow!("loca: {e}"))?;
    fb.add_table(&post).map_err(|e| anyhow!("post: {e}"))?;
    fb.add_table(&name).map_err(|e| anyhow!("name: {e}"))?;
    fb.add_table(&os2).map_err(|e| anyhow!("OS/2: {e}"))?;
    Ok(fb.build())
}

/// A `name` record on the Windows/Unicode-BMP/English platform.
fn name_record(id: NameId, value: &str) -> NameRecord {
    NameRecord::new(3, 1, 0x409, id, value.to_string().into())
}

/// Parse a one-glyph-per-line RMG2000/ttyp0 C header into `code -> (is_full, rows)`.
fn parse_header(path: &str) -> Result<BTreeMap<u32, (bool, Bitmap)>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
    Ok(text.lines().filter_map(parse_line).collect())
}

/// Parse a `{ code, is_full, { r0, .., r11 } }` line, or `None` if it is not one.
fn parse_line(line: &str) -> Option<(u32, (bool, Bitmap))> {
    let rest = line.trim().strip_prefix('{')?;
    let inner_open = rest.find('{')?;
    let mut head = rest[..inner_open].split(',');
    let code = head.next()?.trim().parse::<u32>().ok()?;
    let full = head.next()?.trim() == "true";
    let inner_close = inner_open + rest[inner_open..].find('}')?;
    let vals = rest[inner_open + 1..inner_close]
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect::<Vec<u16>>();
    let rows = vals.try_into().ok()?;
    Some((code, (full, rows)))
}

/// Build a glyph outline: one clockwise rectangle per horizontal run of set pixels
/// (bit 0 = left, row 0 = top), mapped into font units with the row-9 baseline.
fn glyph_path(full: bool, rows: &Bitmap) -> BezPath {
    let mut path = BezPath::new();
    let width = if full { 12 } else { 6 };
    for r in 0..12i32 {
        let bits = rows[r as usize];
        let mut c = 0i32;
        while c < width {
            if bits & (1u16 << c) == 0 {
                c += 1;
                continue;
            }
            let start = c;
            while c < width && bits & (1u16 << c) != 0 {
                c += 1;
            }
            let (x0, x1) = ((start * PX) as f64, (c * PX) as f64);
            let (y0, y1) = (((11 - r) * PX - BASE) as f64, ((12 - r) * PX - BASE) as f64);
            path.move_to((x0, y0));
            path.line_to((x0, y1));
            path.line_to((x1, y1));
            path.line_to((x1, y0));
            path.close_path();
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELLS: &[(char, Bitmap)] = &[
        ('A', [0, 12, 12, 18, 18, 30, 18, 18, 18, 0, 0, 0]),
        ('R', [0, 14, 18, 18, 14, 10, 10, 18, 18, 0, 0, 0]),
        ('Ő', [36, 18, 12, 18, 18, 18, 18, 18, 12, 0, 0, 0]),
        ('ő', [36, 18, 0, 12, 18, 18, 18, 18, 12, 0, 0, 0]),
        ('Ű', [36, 18, 0, 18, 18, 18, 18, 18, 12, 0, 0, 0]),
        ('ű', [36, 18, 0, 18, 18, 18, 18, 18, 28, 0, 0, 0]),
    ];

    fn assert_cells(bytes: &[u8]) {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).unwrap();
        for &(ch, rows) in CELLS {
            let (metrics, bitmap) = font.rasterize(ch, 12.0);
            assert_eq!(metrics.advance_width, 6.0, "{ch}");
            let mut actual = [0u16; 12];
            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let value = bitmap[y * metrics.width + x];
                    assert!(value == 0 || value == 255, "blurred pixel in {ch}");
                    if value != 0 {
                        let row = 10 - metrics.ymin - metrics.height as i32 + y as i32;
                        let col = metrics.xmin + x as i32;
                        assert!((0..12).contains(&row) && (0..6).contains(&col));
                        actual[row as usize] |= 1 << col;
                    }
                }
            }
            assert_eq!(actual, rows, "pixel grid for {ch}");
        }
    }

    #[test]
    fn outline_conversion_preserves_bitmap_cells_and_hungarian_accents() {
        let glyphs = CELLS
            .iter()
            .map(|&(ch, rows)| (ch as u32, (false, rows)))
            .collect();
        assert_cells(&build_font(&glyphs, "Font Test").unwrap());
    }

    #[test]
    fn shipped_rm2000_font_matches_the_reference_cells() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../amnezia/fonts/rm2000.ttf"
        ));
        assert_cells(bytes);
    }
}
