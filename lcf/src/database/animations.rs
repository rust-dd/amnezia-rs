//! Battle animations (`ChunkData::animations`, `0x13`), using liblcf's
//! `ChunkAnimation`, `ChunkAnimationTiming`, `ChunkAnimationFrame` and `ChunkAnimationCellData`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// Placed sprite-sheet tile; `x`/`y` are signed pixel offsets, `scale` a percent.
/// Tone channels use 0–200 (100 = neutral); transparency uses 0–100 (0 = opaque).
/// Deleted cells set `valid = false` but keep their slot to preserve later indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationCell {
    pub valid: bool,
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

/// Flash/sound cue on a 1-based frame. Empty `se_name` is silent; volume and tempo
/// are percentages, balance 50 is centred. Flash RGB uses 0–31;
/// `flash_scope`: 0 = none, 1 = target, 2 = screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationTiming {
    pub frame: u32,
    pub se_name: String,
    pub se_volume: u32,
    pub se_tempo: u32,
    pub se_balance: u32,
    pub flash_scope: u32,
    pub flash_red: u32,
    pub flash_green: u32,
    pub flash_blue: u32,
    pub flash_power: u32,
}

/// `Battle`/`Battle2` sprite-sheet effect. `scope`: 0 = target, 1 = screen;
/// target `position`: 0 = head, 1 = centre, 2 = feet.
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

/// Omitted flash channels default to full-intensity white on the 5-bit scale.
const TIMING_DEFAULT_FLASH: u32 = 31;

// Sound shares Music's chunk IDs but has no fade-in chunk (0x02).
const SOUND_NAME: u32 = 0x01;
const SOUND_VOLUME: u32 = 0x03;
const SOUND_TEMPO: u32 = 0x04;
const SOUND_BALANCE: u32 = 0x05;

// A `Sound` omits volume/tempo when normal (100) and balance when centred (50).
const SOUND_DEFAULT_VOLUME: u32 = 100;
const SOUND_DEFAULT_TEMPO: u32 = 100;
const SOUND_DEFAULT_BALANCE: u32 = 50;

const FRAME_CELLS: u32 = 0x01;

const CELL_VALID: u32 = 0x01;
const CELL_ID: u32 = 0x02;
const CELL_X: u32 = 0x03;
const CELL_Y: u32 = 0x04;
const CELL_SCALE: u32 = 0x05;
const CELL_TONE_RED: u32 = 0x06;
const CELL_TONE_GREEN: u32 = 0x07;
const CELL_TONE_BLUE: u32 = 0x08;
const CELL_TONE_GRAY: u32 = 0x09;
const CELL_TRANSPARENCY: u32 = 0x0A;

// The editor omits neutral zoom and tone channels.
const CELL_DEFAULT_SCALE: u32 = 100;
const CELL_DEFAULT_TONE: i32 = 100;

/// Parse a frame's `cells` list (`AnimationFrame` chunk `0x01`): a `[count]`
/// header then, per cell, a 1-based index id and a chunk stream. Chunk ids
/// (liblcf `ChunkAnimationCellData`): valid `0x01`, cell_id `0x02`, x `0x03`, y
/// `0x04`, scale `0x05`, tone_red `0x06`, tone_green `0x07`, tone_blue `0x08`,
/// tone_gray `0x09`, transparency `0x0A`. `x`, `y`, and the tones are signed
/// (large varints wrap to negative). Omitted fields take their RM2000 defaults:
/// `valid` true, scale and the tones `100`, everything else `0`.
fn parse_cells(data: &[u8]) -> Result<Vec<AnimationCell>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut cells = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _cell_index = reader.varint()?;
        let mut cell = AnimationCell {
            valid: true,
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
                CELL_VALID => cell.valid = Reader::new(sub_data).varint()? != 0,
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

struct SoundFields {
    name: String,
    volume: u32,
    tempo: u32,
    balance: u32,
}

/// Parse a timing's sound effect from its nested `Sound` struct
/// (`ChunkAnimationTiming` chunk `0x02`): a chunk stream sharing the System
/// `Sound` shape — name `0x01`, volume `0x03`, tempo `0x04`, balance `0x05`.
/// Omitted fields keep the RM2000 defaults (volume/tempo `100`, balance `50`).
fn parse_sound(data: &[u8]) -> Result<SoundFields, LcfError> {
    let mut reader = Reader::new(data);
    let mut sound = SoundFields {
        name: String::new(),
        volume: SOUND_DEFAULT_VOLUME,
        tempo: SOUND_DEFAULT_TEMPO,
        balance: SOUND_DEFAULT_BALANCE,
    };
    loop {
        let sub_id = reader.varint()?;
        if sub_id == 0 {
            break;
        }
        let sub_size = reader.varint()? as usize;
        let sub_data = reader.take(sub_size)?;
        match sub_id {
            SOUND_NAME => sound.name = decode_cp1250(sub_data),
            SOUND_VOLUME => sound.volume = Reader::new(sub_data).varint()?,
            SOUND_TEMPO => sound.tempo = Reader::new(sub_data).varint()?,
            SOUND_BALANCE => sound.balance = Reader::new(sub_data).varint()?,
            _ => {}
        }
    }
    Ok(sound)
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
            se_volume: SOUND_DEFAULT_VOLUME,
            se_tempo: SOUND_DEFAULT_TEMPO,
            se_balance: SOUND_DEFAULT_BALANCE,
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
                TIMING_SE => {
                    let sound = parse_sound(sub_data)?;
                    timing.se_name = sound.name;
                    timing.se_volume = sound.volume;
                    timing.se_tempo = sound.tempo;
                    timing.se_balance = sound.balance;
                }
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

/// Missing anchors default to feet, matching liblcf and the editor.
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
        // Negative offsets use wrapped unsigned varints.
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

        // Omitted balance and flash channels must retain their editor defaults.
        let mut sound = subchunk(0x01, b"Punch");
        sound.extend(subchunk(0x03, &varint(80)));
        sound.extend(subchunk(0x04, &varint(120)));
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
                valid: true,
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
                se_volume: 80,
                se_tempo: 120,
                se_balance: 50,
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
        let cell = element(1, &[]);
        let frames = section(&[element(1, &[subchunk(0x01, &section(&[cell]))])]);
        let anim = element(1, &[subchunk(0x0C, &frames)]);
        let ldb = make_ldb(&[(0x13, section(&[anim]))]);
        let a = &parse_animations(&ldb).unwrap()[0];
        assert!(a.name.is_empty());
        assert!(a.animation_name.is_empty());
        assert_eq!((a.scope, a.position), (0, 2));
        assert_eq!(a.frames.len(), 1);
        assert_eq!(
            a.frames[0].cells[0],
            AnimationCell {
                valid: true,
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
    fn a_deleted_cell_parses_as_invalid() {
        // The editor marks a deleted cell with `valid` (0x01) = 0, keeping its
        // slot so the following cell holds index 1; only the flag distinguishes it.
        let deleted = element(1, &[subchunk(0x01, &varint(0)), subchunk(0x02, &varint(4))]);
        let kept = element(2, &[subchunk(0x02, &varint(7))]);
        let frames = section(&[element(1, &[subchunk(0x01, &section(&[deleted, kept]))])]);
        let anim = element(1, &[subchunk(0x0C, &frames)]);
        let ldb = make_ldb(&[(0x13, section(&[anim]))]);
        let a = &parse_animations(&ldb).unwrap()[0];
        let cells = &a.frames[0].cells;
        assert_eq!(cells.len(), 2);
        assert!(!cells[0].valid);
        assert_eq!(cells[0].cell_id, 4);
        assert!(cells[1].valid);
        assert_eq!(cells[1].cell_id, 7);
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
