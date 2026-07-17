//! Monster (enemy) definitions from the database (`ChunkData::enemies`,
//! `0x0E`). RM2000 stores each battle stat as a single scalar chunk, unlike the
//! per-level curves actors use, and adds two per-id resistance-rank byte vectors
//! (elements, states) and a battle-AI action list. Chunk ids follow liblcf
//! `ChunkEnemy` / `ChunkEnemyAction`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// One entry in a monster's battle-AI list. Each turn RM2000 walks the list and
/// performs the highest-`priority` action whose condition currently holds.
///
/// `kind` selects the action family: `0` a basic action (refined by `basic`:
/// `0` attack, `1` dual attack, `2` defend, `3` observe, `4` charge, `5`
/// self-destruct, `6` escape, `7` do nothing), `1` cast the skill named by
/// `skill_id`, `2` transform into the monster named by `enemy_id`.
///
/// `condition_type` gates the action: `0` always, `1` a switch, `2` turn number,
/// `3` this monster's HP%, `4` the party's HP%, `5` the party's average level,
/// `6` party exhausted. `condition_min`/`condition_max` are that condition's two
/// bounds (turn base and interval, or the low and high end of an HP/level
/// range). `priority` (RM2000 "rating", 0–100) breaks ties between the eligible
/// actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnemyAction {
    pub kind: u32,
    pub basic: u32,
    pub skill_id: u32,
    pub enemy_id: u32,
    pub condition_type: u32,
    pub condition_min: u32,
    pub condition_max: u32,
    pub priority: u32,
}

/// A monster (enemy) definition: the battle-relevant scalar stats, the
/// experience and gold it yields when defeated, its element/state resistance
/// ranks, and its battle-AI action list.
///
/// `attribute_ranks` holds the enemy's damage rank per attribute (element) id
/// and `state_ranks` its affliction-chance rank per state id — one byte each,
/// `0` = A (weak) through `4` = E (resist), the neutral middle rank C being `2`.
/// RM2000 writes these as raw byte vectors truncated past the last non-default
/// entry, so a vector shorter than the database's attribute/state count (and any
/// absent trailing index) means rank C for the remaining ids; the raw stored
/// bytes are kept here unpadded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monster {
    pub id: u32,
    pub name: String,
    pub battler: String,
    pub max_hp: u32,
    pub max_sp: u32,
    pub attack: u32,
    pub defense: u32,
    pub spirit: u32,
    pub agility: u32,
    pub exp: u32,
    pub gold: u32,
    pub attribute_ranks: Vec<u8>,
    pub state_ranks: Vec<u8>,
    pub actions: Vec<EnemyAction>,
}

const MONSTER_SECTION: u32 = 0x0E;
const MONSTER_NAME: u32 = 0x01;
const MONSTER_BATTLER: u32 = 0x02;
const MONSTER_MAX_HP: u32 = 0x04;
const MONSTER_MAX_SP: u32 = 0x05;
const MONSTER_ATTACK: u32 = 0x06;
const MONSTER_DEFENSE: u32 = 0x07;
const MONSTER_SPIRIT: u32 = 0x08;
const MONSTER_AGILITY: u32 = 0x09;
const MONSTER_EXP: u32 = 0x0B;
const MONSTER_GOLD: u32 = 0x0C;

// Each rank vector is a uint8 chunk (states `0x20`, elements `0x22`) preceded by
// a redundant element-count chunk (`0x1F` / `0x21`) that we skip: the data
// chunk's own byte length already gives the number of ranks.
const MONSTER_STATE_RANKS: u32 = 0x20;
const MONSTER_ATTRIBUTE_RANKS: u32 = 0x22;
const MONSTER_ACTIONS: u32 = 0x2A;

const ACTION_KIND: u32 = 0x01;
const ACTION_BASIC: u32 = 0x02;
const ACTION_SKILL_ID: u32 = 0x03;
const ACTION_ENEMY_ID: u32 = 0x04;
const ACTION_CONDITION_TYPE: u32 = 0x05;
const ACTION_CONDITION_MIN: u32 = 0x06;
const ACTION_CONDITION_MAX: u32 = 0x07;
const ACTION_PRIORITY: u32 = 0x0D;
const ACTION_DEFAULT_PRIORITY: u32 = 50;

/// Parse a monster's `actions` array (`0x2A`): a `[count]` header then, per
/// action, a 1-based index id and a chunk stream (kind `0x01`, basic `0x02`,
/// skill_id `0x03`, enemy_id `0x04`, condition_type `0x05`, condition_min
/// `0x06`, condition_max `0x07`, priority `0x0D`), matching the nested
/// struct-list shape used elsewhere in the LCF. Switch-effect sub-chunks are
/// skipped. Omitted fields default to 0 except `priority`, which defaults to the
/// RM2000 neutral rating of 50.
fn parse_actions(data: &[u8]) -> Result<Vec<EnemyAction>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut actions = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _action_id = reader.varint()?;
        let mut action = EnemyAction {
            kind: 0,
            basic: 0,
            skill_id: 0,
            enemy_id: 0,
            condition_type: 0,
            condition_min: 0,
            condition_max: 0,
            priority: ACTION_DEFAULT_PRIORITY,
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                ACTION_KIND => action.kind = Reader::new(sub_data).varint()?,
                ACTION_BASIC => action.basic = Reader::new(sub_data).varint()?,
                ACTION_SKILL_ID => action.skill_id = Reader::new(sub_data).varint()?,
                ACTION_ENEMY_ID => action.enemy_id = Reader::new(sub_data).varint()?,
                ACTION_CONDITION_TYPE => action.condition_type = Reader::new(sub_data).varint()?,
                ACTION_CONDITION_MIN => action.condition_min = Reader::new(sub_data).varint()?,
                ACTION_CONDITION_MAX => action.condition_max = Reader::new(sub_data).varint()?,
                ACTION_PRIORITY => action.priority = Reader::new(sub_data).varint()?,
                _ => {}
            }
        }
        actions.push(action);
    }
    Ok(actions)
}

/// Parse the enemy table (`ChunkData::enemies` = `0x0E`) out of an LDB byte
/// slice. Chunk ids (liblcf `ChunkEnemy`): name `0x01`, max_hp `0x04`, max_sp
/// `0x05`, attack `0x06`, defense `0x07`, spirit `0x08`, agility `0x09`, exp
/// `0x0B`, gold `0x0C`, state_ranks `0x20`, attribute_ranks `0x22`, actions
/// `0x2A`. Each stat is a scalar integer chunk and the ranks are raw byte
/// vectors; omitted fields default to 0 / an empty vector.
pub fn parse_monsters(bytes: &[u8]) -> Result<Vec<Monster>, LcfError> {
    let section = find_section(bytes, MONSTER_SECTION, LcfError::MissingMonsters)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut monsters = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut monster = Monster {
            id,
            name: String::new(),
            battler: String::new(),
            max_hp: 0,
            max_sp: 0,
            attack: 0,
            defense: 0,
            spirit: 0,
            agility: 0,
            exp: 0,
            gold: 0,
            attribute_ranks: Vec::new(),
            state_ranks: Vec::new(),
            actions: Vec::new(),
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                MONSTER_NAME => monster.name = decode_cp1250(sub_data),
                MONSTER_BATTLER => monster.battler = decode_cp1250(sub_data),
                MONSTER_MAX_HP => monster.max_hp = Reader::new(sub_data).varint()?,
                MONSTER_MAX_SP => monster.max_sp = Reader::new(sub_data).varint()?,
                MONSTER_ATTACK => monster.attack = Reader::new(sub_data).varint()?,
                MONSTER_DEFENSE => monster.defense = Reader::new(sub_data).varint()?,
                MONSTER_SPIRIT => monster.spirit = Reader::new(sub_data).varint()?,
                MONSTER_AGILITY => monster.agility = Reader::new(sub_data).varint()?,
                MONSTER_EXP => monster.exp = Reader::new(sub_data).varint()?,
                MONSTER_GOLD => monster.gold = Reader::new(sub_data).varint()?,
                MONSTER_STATE_RANKS => monster.state_ranks = sub_data.to_vec(),
                MONSTER_ATTRIBUTE_RANKS => monster.attribute_ranks = sub_data.to_vec(),
                MONSTER_ACTIONS => monster.actions = parse_actions(sub_data)?,
                _ => {}
            }
        }
        monsters.push(monster);
    }
    Ok(monsters)
}

#[cfg(test)]
mod tests {
    use super::EnemyAction;
    use crate::test_util::{element, make_ldb, section, subchunk, varint};
    use crate::{LcfError, Monster, parse_monsters};

    #[test]
    fn parses_monster_battle_stats() {
        // Name bytes are CP1250 "Sárkány" (dragon): 0xE1 = 'á'.
        let dragon = element(
            1,
            &[
                subchunk(0x01, &[0x53, 0xE1, 0x72, 0x6B, 0xE1, 0x6E, 0x79]),
                subchunk(0x02, b"Dragon1"),
                subchunk(0x04, &varint(999)),
                subchunk(0x05, &varint(120)),
                subchunk(0x06, &varint(180)),
                subchunk(0x07, &varint(90)),
                subchunk(0x08, &varint(70)),
                subchunk(0x09, &varint(45)),
                subchunk(0x0B, &varint(1500)),
                subchunk(0x0C, &varint(800)),
            ],
        );
        let ldb = make_ldb(&[(0x0D, vec![1]), (0x0E, section(&[dragon]))]);
        let monsters = parse_monsters(&ldb).unwrap();
        assert_eq!(monsters.len(), 1);
        assert_eq!(
            monsters[0],
            Monster {
                id: 1,
                name: "Sárkány".to_string(),
                battler: "Dragon1".to_string(),
                max_hp: 999,
                max_sp: 120,
                attack: 180,
                defense: 90,
                spirit: 70,
                agility: 45,
                exp: 1500,
                gold: 800,
                attribute_ranks: vec![],
                state_ranks: vec![],
                actions: vec![],
            }
        );
    }

    #[test]
    fn monster_fields_default_to_zero_when_omitted() {
        let slime = element(2, &[subchunk(0x01, b"Slime"), subchunk(0x04, &varint(30))]);
        let ldb = make_ldb(&[(0x0E, section(&[slime]))]);
        let monsters = parse_monsters(&ldb).unwrap();
        let m = &monsters[0];
        assert_eq!(m.id, 2);
        assert_eq!(m.name, "Slime");
        assert_eq!(m.max_hp, 30);
        assert_eq!((m.attack, m.defense, m.spirit, m.agility), (0, 0, 0, 0));
        assert_eq!((m.exp, m.gold), (0, 0));
        assert!(m.attribute_ranks.is_empty());
        assert!(m.state_ranks.is_empty());
        assert!(m.actions.is_empty());
    }

    #[test]
    fn parses_monster_ranks_and_actions() {
        // Two AI entries: an always-on basic attack (priority defaults to 50),
        // then a skill cast gated on this monster's HP dropping to 1..=25%.
        let attack = element(1, &[subchunk(0x01, &varint(0)), subchunk(0x02, &varint(0))]);
        let skill = element(
            2,
            &[
                subchunk(0x01, &varint(1)),  // kind: skill
                subchunk(0x03, &varint(42)), // skill_id
                subchunk(0x05, &varint(3)),  // condition_type: monster-hp%
                subchunk(0x06, &varint(1)),  // condition_min
                subchunk(0x07, &varint(25)), // condition_max
                subchunk(0x0D, &varint(80)), // priority
            ],
        );
        let mut actions = varint(2);
        actions.extend_from_slice(&attack);
        actions.extend_from_slice(&skill);
        let golem = element(
            3,
            &[
                subchunk(0x01, b"Golem"),
                subchunk(0x04, &varint(400)),
                subchunk(0x1F, &varint(4)),    // state_ranks_size, skipped
                subchunk(0x20, &[4, 4, 4, 2]), // state_ranks: immune x3, then C
                subchunk(0x21, &varint(3)),    // attribute_ranks_size, skipped
                subchunk(0x22, &[0, 2, 4]),    // attribute_ranks: A(weak), C, E(resist)
                subchunk(0x2A, &actions),
            ],
        );
        let ldb = make_ldb(&[(0x0E, section(&[golem]))]);
        let monsters = parse_monsters(&ldb).unwrap();
        let m = &monsters[0];
        assert_eq!(m.id, 3);
        assert_eq!(m.name, "Golem");
        assert_eq!(m.max_hp, 400);
        assert_eq!(m.state_ranks, vec![4, 4, 4, 2]);
        assert_eq!(m.attribute_ranks, vec![0, 2, 4]);
        assert_eq!(m.actions.len(), 2);
        assert_eq!(
            m.actions[0],
            EnemyAction {
                kind: 0,
                basic: 0,
                skill_id: 0,
                enemy_id: 0,
                condition_type: 0,
                condition_min: 0,
                condition_max: 0,
                priority: 50,
            }
        );
        assert_eq!(
            m.actions[1],
            EnemyAction {
                kind: 1,
                basic: 0,
                skill_id: 42,
                enemy_id: 0,
                condition_type: 3,
                condition_min: 1,
                condition_max: 25,
                priority: 80,
            }
        );
    }

    #[test]
    fn parse_monsters_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(
            parse_monsters(&ldb),
            Err(LcfError::MissingMonsters)
        ));
    }
}
