//! State (status condition) definitions from the database
//! (`ChunkData::states`, `0x12`). A state is a battle status — KO, Poison,
//! Sleep, Berserk — that changes how a battler may act (`restriction`), how it
//! wears off (`hold_turn`, `auto_release_prob`, `release_by_damage`), and
//! whether it drains or regenerates HP each turn (`hp_change_*`). Chunk ids
//! follow liblcf `ChunkState`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// A state (status condition) definition. `restriction` limits the battler's
/// actions while the state holds: `0` none (acts normally), `1` do nothing
/// (can't act), `2` attack an enemy at random (berserk), `3` attack an ally at
/// random (confusion). `priority` (0–100) picks which state's graphic and
/// restriction dominate when several are active at once.
///
/// Recovery is governed by three odds: `hold_turn`, the minimum number of turns
/// the state is held before it can lift; `auto_release_prob`, the percent chance
/// per turn to lift once those turns have passed; and `release_by_damage`, the
/// percent chance to lift when the battler is hit by a physical attack. All
/// three default to `0` (a state that never lifts on its own, like KO or
/// Poison).
///
/// `hp_change_type` says how an HP-changing state moves HP (liblcf
/// `ChangeType`: `0` lose, `1` gain, `2` nothing). Each battle turn the battler
/// loses or gains `hp_change_val` flat points plus `hp_change_max` percent of
/// its max HP, while `hp_change_map_steps`/`hp_change_map_val` drain it on the
/// map (`hp_change_map_val` HP per `hp_change_map_steps` steps walked). All five
/// default to `0` (a zero-amount no-op); Poison sets them to bleed HP each turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub rates: [u32; 5],
    pub persistence: u32,
    pub id: u32,
    pub name: String,
    pub restriction: u32,
    pub priority: u32,
    pub hold_turn: u32,
    pub auto_release_prob: u32,
    pub release_by_damage: u32,
    pub hp_change_type: u32,
    pub hp_change_max: u32,
    pub hp_change_val: u32,
    pub hp_change_map_steps: u32,
    pub hp_change_map_val: u32,
}

const STATE_SECTION: u32 = 0x12;
const STATE_NAME: u32 = 0x01;
const STATE_PRIORITY: u32 = 0x04;
const STATE_RESTRICTION: u32 = 0x05;
const STATE_HOLD_TURN: u32 = 0x15;
const STATE_AUTO_RELEASE_PROB: u32 = 0x16;
const STATE_RELEASE_BY_DAMAGE: u32 = 0x17;
const STATE_HP_CHANGE_TYPE: u32 = 0x2D;
const STATE_HP_CHANGE_MAX: u32 = 0x3D;
const STATE_HP_CHANGE_VAL: u32 = 0x3E;
const STATE_HP_CHANGE_MAP_STEPS: u32 = 0x3F;
const STATE_HP_CHANGE_MAP_VAL: u32 = 0x40;

// RM2000 omits a state's priority when it equals the editor default of 50; the
// death state (id 1) leaves its name to the System vocabulary and so stores no
// name chunk, which decodes to an empty string here.
const STATE_DEFAULT_PRIORITY: u32 = 50;

/// Parse the state table (`ChunkData::states` = `0x12`) out of an LDB byte
/// slice. Chunk ids (liblcf `ChunkState`): name `0x01`, priority `0x04`,
/// restriction `0x05`, hold_turn `0x15`, auto_release_prob `0x16`,
/// release_by_damage `0x17`, hp_change_type `0x2D`, hp_change_max `0x3D`,
/// hp_change_val `0x3E`, hp_change_map_steps `0x3F`, hp_change_map_val `0x40`.
/// Each numeric field is a scalar integer chunk; omitted recovery odds and
/// hp-change fields default to 0 and omitted priority to 50.
pub fn parse_states(bytes: &[u8]) -> Result<Vec<State>, LcfError> {
    let section = find_section(bytes, STATE_SECTION, LcfError::MissingStates)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut states = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut state = State {
            rates: [100, 80, 60, 30, 0],
            persistence: 0,
            id,
            name: String::new(),
            restriction: 0,
            priority: STATE_DEFAULT_PRIORITY,
            hold_turn: 0,
            auto_release_prob: 0,
            release_by_damage: 0,
            hp_change_type: 0,
            hp_change_max: 0,
            hp_change_val: 0,
            hp_change_map_steps: 0,
            hp_change_map_val: 0,
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                0x0B..=0x0F => {
                    state.rates[(sub_id - 0x0B) as usize] = Reader::new(sub_data).varint()?
                }
                STATE_NAME => state.name = decode_cp1250(sub_data),
                0x02 => state.persistence = Reader::new(sub_data).varint()?,
                STATE_PRIORITY => state.priority = Reader::new(sub_data).varint()?,
                STATE_RESTRICTION => state.restriction = Reader::new(sub_data).varint()?,
                STATE_HOLD_TURN => state.hold_turn = Reader::new(sub_data).varint()?,
                STATE_AUTO_RELEASE_PROB => {
                    state.auto_release_prob = Reader::new(sub_data).varint()?
                }
                STATE_RELEASE_BY_DAMAGE => {
                    state.release_by_damage = Reader::new(sub_data).varint()?
                }
                STATE_HP_CHANGE_TYPE => state.hp_change_type = Reader::new(sub_data).varint()?,
                STATE_HP_CHANGE_MAX => state.hp_change_max = Reader::new(sub_data).varint()?,
                STATE_HP_CHANGE_VAL => state.hp_change_val = Reader::new(sub_data).varint()?,
                STATE_HP_CHANGE_MAP_STEPS => {
                    state.hp_change_map_steps = Reader::new(sub_data).varint()?
                }
                STATE_HP_CHANGE_MAP_VAL => {
                    state.hp_change_map_val = Reader::new(sub_data).varint()?
                }
                _ => {}
            }
        }
        states.push(state);
    }
    Ok(states)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{element, make_ldb, section, subchunk, varint};
    use crate::{LcfError, State, parse_states};

    #[test]
    fn parses_state_restriction_and_recovery() {
        // Sleep: held 1 turn, 25% per-turn wake-up, 50% wake-up when hit.
        let sleep = element(
            1,
            &[
                subchunk(0x01, b"Alvas"),
                subchunk(0x04, &varint(55)),
                subchunk(0x05, &varint(1)),
                subchunk(0x15, &varint(1)),
                subchunk(0x16, &varint(25)),
                subchunk(0x17, &varint(50)),
            ],
        );
        let ldb = make_ldb(&[(0x11, section(&[])), (0x12, section(&[sleep]))]);
        let states = parse_states(&ldb).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            State {
                rates: [100, 80, 60, 30, 0],
                persistence: 0,
                id: 1,
                name: "Alvas".to_string(),
                restriction: 1,
                priority: 55,
                hold_turn: 1,
                auto_release_prob: 25,
                release_by_damage: 50,
                hp_change_type: 0,
                hp_change_max: 0,
                hp_change_val: 0,
                hp_change_map_steps: 0,
                hp_change_map_val: 0,
            }
        );
    }

    #[test]
    fn parses_hp_change_block() {
        // An explicit per-turn HP-change block parses its three scalar fields:
        // type (0x2D), max-percent (0x3D), flat val (0x3E). The map-step fields
        // are omitted, so they stay 0.
        let bleeder = element(
            2,
            &[
                subchunk(0x01, b"Mereg"),
                subchunk(0x2D, &varint(1)),
                subchunk(0x3D, &varint(10)),
                subchunk(0x3E, &varint(5)),
            ],
        );
        let ldb = make_ldb(&[(0x12, section(&[bleeder]))]);
        let state = &parse_states(&ldb).unwrap()[0];
        assert_eq!(state.hp_change_type, 1);
        assert_eq!(state.hp_change_max, 10);
        assert_eq!(state.hp_change_val, 5);
        assert_eq!(
            (state.hp_change_map_steps, state.hp_change_map_val),
            (0, 0),
            "omitted map-step drain defaults to 0"
        );
    }

    #[test]
    fn parses_berserk_restriction() {
        // Name bytes are CP1250 "Őrült" (crazy): 0xD5 = 'Ő', 0xFC = 'ü'.
        // Confusion attacks allies at random — restriction 3.
        let confusion = element(
            2,
            &[
                subchunk(0x01, &[0xD5, 0x72, 0xFC, 0x6C, 0x74]),
                subchunk(0x05, &varint(3)),
                subchunk(0x16, &varint(30)),
                subchunk(0x17, &varint(25)),
            ],
        );
        let ldb = make_ldb(&[(0x12, section(&[confusion]))]);
        let state = &parse_states(&ldb).unwrap()[0];
        assert_eq!(state.name, "Őrült");
        assert_eq!(state.restriction, 3, "attacks an ally at random");
        assert_eq!(state.priority, 50, "omitted priority defaults to 50");
        assert_eq!(state.hold_turn, 0, "omitted hold_turn defaults to 0");
        assert_eq!(state.auto_release_prob, 30);
        assert_eq!(state.release_by_damage, 25);
    }

    #[test]
    fn death_state_defaults_when_fields_omitted() {
        // The death state (id 1) omits its name and every recovery odd: it can't
        // act (restriction 1), holds highest priority, and never lifts on its own.
        let death = element(
            1,
            &[subchunk(0x04, &varint(100)), subchunk(0x05, &varint(1))],
        );
        let ldb = make_ldb(&[(0x12, section(&[death]))]);
        let state = &parse_states(&ldb).unwrap()[0];
        assert_eq!(state.id, 1);
        assert!(state.name.is_empty());
        assert_eq!(state.restriction, 1);
        assert_eq!(state.priority, 100);
        assert_eq!(
            (
                state.hold_turn,
                state.auto_release_prob,
                state.release_by_damage
            ),
            (0, 0, 0)
        );
    }

    #[test]
    fn parse_states_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(parse_states(&ldb), Err(LcfError::MissingStates)));
    }

    #[test]
    fn state_rates_preserve_defaults_and_explicit_zero() {
        let chunks = [0, 85, 70, 40, 0]
            .into_iter()
            .enumerate()
            .map(|(i, value)| subchunk(0x0B + i as u32, &varint(value)))
            .collect::<Vec<_>>();
        let ldb = make_ldb(&[(0x12, section(&[element(1, &[]), element(2, &chunks)]))]);
        let states = parse_states(&ldb).unwrap();
        assert_eq!(states[0].rates, [100, 80, 60, 30, 0]);
        assert_eq!(states[1].rates, [0, 85, 70, 40, 0]);
    }
}
