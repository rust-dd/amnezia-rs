//! Convert EasyRPG's RMG2000 bitmap font (the free reproduction of RPG Maker
//! 2000's built-in font) into a TrueType outline font the Bevy text pipeline can
//! load. RPG Maker 2000 ships no font of its own — RPG_RT draws text with a
//! built-in bitmap face — so the faithful font is EasyRPG's reproduction of it.
//!
//! Each RMG2000 glyph is 12 rows of a `u16` bitmask (bit 0 = leftmost column, row
//! 0 = top); half-width glyphs are 6px wide, full-width 12px. RMG2000 letters rest
//! with their body bottom on row 9, so the baseline sits there. RMG2000 lacks the
//! Hungarian double-acute letters (ő ű Ő Ű); those come from EasyRPG's ttyp0
//! fallback, shifted down one row to match its (one-pixel-higher) baseline. The
//! menu cursor, arrows, and em dash the chrome renders as text are drawn in, since
//! RM2000 renders those graphically rather than as characters.
//!
//! Run from the workspace root; needs the gitignored EasyRPG reference checkout:
//!     cargo run -p amnezia-convert --bin build_font
//! Writes amnezia/fonts/rmg2000.ttf (committed, so a normal checkout needs neither
//! this tool nor the reference).

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
/// Hungarian double-acute letters pulled from ttyp0 (RMG2000 has none): Ő ő Ű ű.
const TTYP0_FILL: &[u32] = &[336, 337, 368, 369];

type Bitmap = [u16; 12];

fn main() -> Result<()> {
    let mut glyphs = parse_header(&format!("{REF}/bitmapfont_rmg2000.h"))?;
    let ttyp0 = parse_header(&format!("{REF}/bitmapfont_ttyp0.h"))?;
    for &code in TTYP0_FILL {
        if let (false, Some(&(full, rows))) = (glyphs.contains_key(&code), ttyp0.get(&code)) {
            // Shift down one row so the body's baseline aligns with RMG2000.
            let mut shifted = [0u16; 12];
            shifted[1..].copy_from_slice(&rows[..11]);
            glyphs.insert(code, (full, shifted));
        }
    }
    for &(code, rows) in DRAWN {
        glyphs.insert(code, (false, rows));
    }
    glyphs.entry(32).or_insert((false, [0; 12]));

    let mut builder = GlyfLocaBuilder::new();
    let mut mappings: Vec<(char, GlyphId)> = Vec::new();
    let mut metrics: Vec<LongMetric> = vec![LongMetric::new((6 * PX) as u16, 0)];
    // Glyph id 0 is `.notdef`, an empty glyph.
    builder
        .add_glyph(&Glyph::Simple(SimpleGlyph::default()))
        .map_err(|e| anyhow!("notdef: {e}"))?;

    let mut gid: u16 = 1;
    for (&code, &(full, rows)) in &glyphs {
        if code < 32 {
            continue; // 1-31 are RM2000 control/icon glyphs
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
        metrics.push(LongMetric::new((if full { 12 } else { 6 } * PX) as u16, 0));
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
        name_record(NameId::FAMILY_NAME, "RMG2000"),
        name_record(NameId::SUBFAMILY_NAME, "Regular"),
        name_record(NameId::FULL_NAME, "RMG2000"),
        name_record(NameId::POSTSCRIPT_NAME, "RMG2000"),
    ]);

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
    let bytes = fb.build();
    std::fs::write("amnezia/fonts/rmg2000.ttf", &bytes).context("write ttf")?;
    println!(
        "wrote amnezia/fonts/rmg2000.ttf ({num_glyphs} glyphs, {} bytes)",
        bytes.len()
    );
    Ok(())
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
    let code: u32 = head.next()?.trim().parse().ok()?;
    let full = head.next()?.trim() == "true";
    let inner_close = inner_open + rest[inner_open..].find('}')?;
    let vals: Vec<u16> = rest[inner_open + 1..inner_close]
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let rows: Bitmap = vals.try_into().ok()?;
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
