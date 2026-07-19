//! Battle-animation definitions from the database (`ChunkData::animations`,
//! `0x13`). An animation is the sprite-sheet effect RM2000 overlays on a battler
//! when a skill or attack lands: a `Battle`/`Battle2` graphic, the per-frame
//! placement of its tiles (cells), and a timeline of flashes and sound effects.
//! Chunk ids follow liblcf `ChunkAnimation` / `ChunkAnimationTiming` /
//! `ChunkAnimationFrame` / `ChunkAnimationCellData`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// One placed sprite-sheet tile within an animation frame. `cell_id` selects the
/// tile from the animation's graphic (a 5x5 grid of patterns, so `0..=24`).
/// `x`/`y` offset it from the animation's anchor in screen pixels (signed,
/// centred on 0). `scale` is a zoom percent (`100` = full size). The four
/// `tone_*` channels tint the tile on RM2000's `0..=200` scale (`100` = neutral)
/// and `transparency` is a `0..=100` percent (`0` = opaque).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationCell {
    pub cell_id: u32,
    pub x: i32,
    pub y: i32,
    pub scale: u32,
    pub tone_red: i32,
    pub tone_green: i32,
    pub tone_blue: i32,
    pub tone_gray: i32,
    pub transparency: u32,
}

/// One frame of an animation: the sprite-sheet cells drawn together for that
/// tick of the effect. RM2000 plays a frame list in order at a fixed rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationFrame {
    pub cells: Vec<AnimationCell>,
}

/// A frame-timed flash and sound effect on an animation's timeline. `frame` is
/// the 1-based frame the effect fires on and `se_name` the sound-effect file
/// under `Sound/` (empty = silent). `flash_scope` selects what flashes (`0`
/// nothing, `1` the target, `2` the whole screen); `flash_red`/`green`/`blue`
/// are the flash colour on RM2000's `0..=31` scale and `flash_power` its
/// strength.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationTiming {
    pub frame: u32,
    pub se_name: String,
    pub flash_scope: u32,
    pub flash_red: u32,
    pub flash_green: u32,
    pub flash_blue: u32,
    pub flash_power: u32,
}

/// A battle-animation definition: the sprite-sheet effect played when a skill or
/// attack resolves. `animation_name` is the `Battle`/`Battle2` graphic base
/// name, `scope` whether the effect covers a single target (`0`) or the whole
/// screen (`1`), and `position` its vertical anchor on the target (`0` head, `1`
/// centre, `2` feet). `frames` are the per-tick cell placements and `timings`
/// the flash / sound timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
    pub id: u32,
    pub name: String,
    pub animation_name: String,
    pub scope: u32,
    pub position: u32,
    pub frames: Vec<AnimationFrame>,
    pub timings: Vec<AnimationTiming>,
}

const ANIMATION_SECTION: u32 = 0x13;
const ANIMATION_NAME: u32 = 0x01;
const ANIMATION_GRAPHIC: u32 = 0x02;
const ANIMATION_TIMINGS: u32 = 0x06;
const ANIMATION_SCOPE: u32 = 0x09;
const ANIMATION_POSITION: u32 = 0x0A;
const ANIMATION_FRAMES: u32 = 0x0C;

const TIMING_FRAME: u32 = 0x01;
const TIMING_SE: u32 = 0x02;
const TIMING_FLASH_SCOPE: u32 = 0x03;
const TIMING_FLASH_RED: u32 = 0x04;
const TIMING_FLASH_GREEN: u32 = 0x05;
const TIMING_FLASH_BLUE: u32 = 0x06;
const TIMING_FLASH_POWER: u32 = 0x07;

// RM2000 omits a flash channel that equals the editor default of full-intensity
// white (31 on the 5-bit flash scale), so absent colours restore 31.
const TIMING_DEFAULT_FLASH: u32 = 31;

const SOUND_NAME: u32 = 0x01;

const FRAME_CELLS: u32 = 0x01;

// Cell chunk ids trail liblcf's field order by one: `0x01` is an always-zero
// leading flag this game never sets, so the nine drawn fields start at `0x02`.
const CELL_ID: u32 = 0x02;
const CELL_X: u32 = 0x03;
const CELL_Y: u32 = 0x04;
const CELL_SCALE: u32 = 0x05;
const CELL_TONE_RED: u32 = 0x06;
const CELL_TONE_GREEN: u32 = 0x07;
const CELL_TONE_BLUE: u32 = 0x08;
const CELL_TONE_GRAY: u32 = 0x09;
const CELL_TRANSPARENCY: u32 = 0x0A;

// Zoom and each tone channel are omitted when neutral (100); the rest default to
// 0.
const CELL_DEFAULT_SCALE: u32 = 100;
const CELL_DEFAULT_TONE: i32 = 100;

/// Parse a frame's `cells` list (`AnimationFrame` chunk `0x01`): a `[count]`
/// header then, per cell, a 1-based index id and a chunk stream. Chunk ids
/// (liblcf `ChunkAnimationCellData`, shifted past the leading `0x01` flag):
/// cell_id `0x02`, x `0x03`, y `0x04`, scale `0x05`, tone_red `0x06`, tone_green
/// `0x07`, tone_blue `0x08`, tone_gray `0x09`, transparency `0x0A`. `x`, `y`, and
/// the tones are signed (large varints wrap to negative). Omitted fields take
/// their RM2000 defaults: scale and the tones `100`, everything else `0`.
fn parse_cells(data: &[u8]) -> Result<Vec<AnimationCell>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut cells = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _cell_index = reader.varint()?;
        let mut cell = AnimationCell {
            cell_id: 0,
            x: 0,
            y: 0,
            scale: CELL_DEFAULT_SCALE,
            tone_red: CELL_DEFAULT_TONE,
            tone_green: CELL_DEFAULT_TONE,
            tone_blue: CELL_DEFAULT_TONE,
            tone_gray: CELL_DEFAULT_TONE,
            transparency: 0,
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                CELL_ID => cell.cell_id = Reader::new(sub_data).varint()?,
                CELL_X => cell.x = Reader::new(sub_data).varint()? as i32,
                CELL_Y => cell.y = Reader::new(sub_data).varint()? as i32,
                CELL_SCALE => cell.scale = Reader::new(sub_data).varint()?,
                CELL_TONE_RED => cell.tone_red = Reader::new(sub_data).varint()? as i32,
                CELL_TONE_GREEN => cell.tone_green = Reader::new(sub_data).varint()? as i32,
                CELL_TONE_BLUE => cell.tone_blue = Reader::new(sub_data).varint()? as i32,
                CELL_TONE_GRAY => cell.tone_gray = Reader::new(sub_data).varint()? as i32,
                CELL_TRANSPARENCY => cell.transparency = Reader::new(sub_data).varint()?,
                _ => {}
            }
        }
        cells.push(cell);
    }
    Ok(cells)
}

/// Parse an animation's `frames` list (`ChunkAnimation` chunk `0x0C`): a
/// `[count]` header then, per frame, a 1-based index id and a chunk stream whose
/// sole member is the cell list (`AnimationFrame` chunk `0x01`).
fn parse_frames(data: &[u8]) -> Result<Vec<AnimationFrame>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut frames = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _frame_index = reader.varint()?;
        let mut cells = Vec::new();
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            if sub_id == FRAME_CELLS {
                cells = parse_cells(sub_data)?;
            }
        }
        frames.push(AnimationFrame { cells });
    }
    Ok(frames)
}

/// Extract a timing's sound-effect file name from its nested `Sound` struct
/// (`ChunkAnimationTiming` chunk `0x02`): a chunk stream whose name member is
/// `ChunkSound` `0x01`. Volume, tempo, and balance are not needed for playback
/// wiring and are skipped.
fn parse_sound_name(data: &[u8]) -> Result<String, LcfError> {
    let mut reader = Reader::new(data);
    let mut name = String::new();
    loop {
        let sub_id = reader.varint()?;
        if sub_id == 0 {
            break;
        }
        let sub_size = reader.varint()? as usize;
        let sub_data = reader.take(sub_size)?;
        if sub_id == SOUND_NAME {
            name = decode_cp1250(sub_data);
        }
    }
    Ok(name)
}

/// Parse an animation's `timings` list (`ChunkAnimation` chunk `0x06`): a
/// `[count]` header then, per timing, a 1-based index id and a chunk stream.
/// Chunk ids (liblcf `ChunkAnimationTiming`): frame `0x01`, se `0x02` (a nested
/// `Sound`), flash_scope `0x03`, flash_red `0x04`, flash_green `0x05`,
/// flash_blue `0x06`, flash_power `0x07`. Omitted flash channels default to the
/// RM2000 full-intensity `31`.
fn parse_timings(data: &[u8]) -> Result<Vec<AnimationTiming>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut timings = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _timing_index = reader.varint()?;
        let mut timing = AnimationTiming {
            frame: 0,
            se_name: String::new(),
            flash_scope: 0,
            flash_red: TIMING_DEFAULT_FLASH,
            flash_green: TIMING_DEFAULT_FLASH,
            flash_blue: TIMING_DEFAULT_FLASH,
            flash_power: TIMING_DEFAULT_FLASH,
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                TIMING_FRAME => timing.frame = Reader::new(sub_data).varint()?,
                TIMING_SE => timing.se_name = parse_sound_name(sub_data)?,
                TIMING_FLASH_SCOPE => timing.flash_scope = Reader::new(sub_data).varint()?,
                TIMING_FLASH_RED => timing.flash_red = Reader::new(sub_data).varint()?,
                TIMING_FLASH_GREEN => timing.flash_green = Reader::new(sub_data).varint()?,
                TIMING_FLASH_BLUE => timing.flash_blue = Reader::new(sub_data).varint()?,
                TIMING_FLASH_POWER => timing.flash_power = Reader::new(sub_data).varint()?,
                _ => {}
            }
        }
        timings.push(timing);
    }
    Ok(timings)
}

// liblcf defaults an animation's vertical anchor to `2` (down / feet) when the
// `position` chunk is absent — the same value the RM2000 editor writes for a new
// animation.
const ANIMATION_DEFAULT_POSITION: u32 = 2;

/// Parse the battle-animation table (`ChunkData::animations` = `0x13`) out of an
/// LDB byte slice. Chunk ids (liblcf `ChunkAnimation`): name `0x01`,
/// animation_name `0x02`, timings `0x06`, scope `0x09`, position `0x0A`, frames
/// `0x0C`. Names decode from Windows-1250; an omitted scope defaults to `0`
/// (single target) and an omitted position to `2` (feet), matching liblcf.
pub fn parse_animations(bytes: &[u8]) -> Result<Vec<Animation>, LcfError> {
    let section = find_section(bytes, ANIMATION_SECTION, LcfError::MissingAnimations)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut animations = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut animation = Animation {
            id,
            name: String::new(),
            animation_name: String::new(),
            scope: 0,
            position: ANIMATION_DEFAULT_POSITION,
            frames: Vec::new(),
            timings: Vec::new(),
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                ANIMATION_NAME => animation.name = decode_cp1250(sub_data),
                ANIMATION_GRAPHIC => animation.animation_name = decode_cp1250(sub_data),
                ANIMATION_TIMINGS => animation.timings = parse_timings(sub_data)?,
                ANIMATION_SCOPE => animation.scope = Reader::new(sub_data).varint()?,
                ANIMATION_POSITION => animation.position = Reader::new(sub_data).varint()?,
                ANIMATION_FRAMES => animation.frames = parse_frames(sub_data)?,
                _ => {}
            }
        }
        animations.push(animation);
    }
    Ok(animations)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{element, make_ldb, section, subchunk, varint};
    use crate::{AnimationCell, AnimationTiming, LcfError, parse_animations};

    #[test]
    fn parses_animation_frames_cells_and_timings() {
        // One cell: tile 3, offset x=-24 (a signed field stored as a wrapped
        // varint) y=48, zoomed to 200%, 40% transparent, neutral tone.
        let cell = element(
            1,
            &[
                subchunk(0x02, &varint(3)),
                subchunk(0x03, &varint((-24i32) as u32)),
                subchunk(0x04, &varint(48)),
                subchunk(0x05, &varint(200)),
                subchunk(0x0A, &varint(40)),
            ],
        );
        let frames = section(&[element(1, &[subchunk(0x01, &section(&[cell]))])]);

        // A timing at frame 5: plays "Punch" (name-only Sound struct) and flashes
        // the whole screen (scope 2) with red 28, the other channels defaulting.
        let mut sound = subchunk(0x01, b"Punch");
        sound.extend(varint(0));
        let timing = element(
            1,
            &[
                subchunk(0x01, &varint(5)),
                subchunk(0x02, &sound),
                subchunk(0x03, &varint(2)),
                subchunk(0x04, &varint(28)),
            ],
        );
        let timings = section(&[timing]);

        // Name bytes are CP1250 "Tűz" (fire): 0xFB = 'ű'.
        let anim = element(
            1,
            &[
                subchunk(0x01, &[0x54, 0xFB, 0x7A]),
                subchunk(0x02, b"Fire1"),
                subchunk(0x06, &timings),
                subchunk(0x09, &varint(1)),
                subchunk(0x0A, &varint(2)),
                subchunk(0x0C, &frames),
            ],
        );
        let ldb = make_ldb(&[(0x12, section(&[])), (0x13, section(&[anim]))]);
        let animations = parse_animations(&ldb).unwrap();
        assert_eq!(animations.len(), 1);
        let a = &animations[0];
        assert_eq!(a.id, 1);
        assert_eq!(a.name, "Tűz");
        assert_eq!(a.animation_name, "Fire1");
        assert_eq!((a.scope, a.position), (1, 2));
        assert_eq!(a.frames.len(), 1);
        assert_eq!(a.frames[0].cells.len(), 1);
        assert_eq!(
            a.frames[0].cells[0],
            AnimationCell {
                cell_id: 3,
                x: -24,
                y: 48,
                scale: 200,
                tone_red: 100,
                tone_green: 100,
                tone_blue: 100,
                tone_gray: 100,
                transparency: 40,
            }
        );
        assert_eq!(a.timings.len(), 1);
        assert_eq!(
            a.timings[0],
            AnimationTiming {
                frame: 5,
                se_name: "Punch".to_string(),
                flash_scope: 2,
                flash_red: 28,
                flash_green: 31,
                flash_blue: 31,
                flash_power: 31,
            }
        );
    }

    #[test]
    fn animation_cell_and_frame_defaults_apply() {
        // A frame whose single cell stores no fields: every field is a default.
        let cell = element(1, &[]);
        let frames = section(&[element(1, &[subchunk(0x01, &section(&[cell]))])]);
        let anim = element(1, &[subchunk(0x0C, &frames)]);
        let ldb = make_ldb(&[(0x13, section(&[anim]))]);
        let a = &parse_animations(&ldb).unwrap()[0];
        assert!(a.name.is_empty());
        assert!(a.animation_name.is_empty());
        // An omitted scope defaults to 0 (single target); an omitted position to
        // 2 (feet), the liblcf default.
        assert_eq!((a.scope, a.position), (0, 2));
        assert_eq!(a.frames.len(), 1);
        assert_eq!(
            a.frames[0].cells[0],
            AnimationCell {
                cell_id: 0,
                x: 0,
                y: 0,
                scale: 100,
                tone_red: 100,
                tone_green: 100,
                tone_blue: 100,
                tone_gray: 100,
                transparency: 0,
            }
        );
        assert!(a.timings.is_empty());
    }

    #[test]
    fn parse_animations_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(
            parse_animations(&ldb),
            Err(LcfError::MissingAnimations)
        ));
    }
}
