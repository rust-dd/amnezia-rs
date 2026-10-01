//! Vocabulary (`ChunkData::terms`, `0x15`): a bare chunk stream of CP1250 strings.
//! IDs follow liblcf `ChunkTerms`; missing terms stay empty. Only RM2000 fields
//! (`0x01..=0x99`) are read, excluding Maniac (`0xA1+`) and EasyRPG (`0xC8+`) extensions.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// The full RPG Maker 2000 term vocabulary (`rpg::Terms`): every UI and message
/// string the engine substitutes at runtime. Fields mirror liblcf's `rpg::Terms`
/// names one-to-one; an omitted term is the empty string. Battle and reward
/// messages are RM2000 name-concatenation templates (the battler name is prefixed
/// or the value/term suffixed — see EasyRPG `game_message_terms.cpp`), not
/// `%S`-placeholder strings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Terms {
    pub encounter: String,
    pub special_combat: String,
    pub escape_success: String,
    pub escape_failure: String,
    pub victory: String,
    pub defeat: String,
    pub exp_received: String,
    pub gold_recieved_a: String,
    pub gold_recieved_b: String,
    pub item_recieved: String,
    pub attacking: String,
    pub enemy_critical: String,
    pub actor_critical: String,
    pub defending: String,
    pub observing: String,
    pub focus: String,
    pub autodestruction: String,
    pub enemy_escape: String,
    pub enemy_transform: String,
    pub enemy_damaged: String,
    pub enemy_undamaged: String,
    pub actor_damaged: String,
    pub actor_undamaged: String,
    pub skill_failure_a: String,
    pub skill_failure_b: String,
    pub skill_failure_c: String,
    pub dodge: String,
    pub use_item: String,
    pub hp_recovery: String,
    pub parameter_increase: String,
    pub parameter_decrease: String,
    pub enemy_hp_absorbed: String,
    pub actor_hp_absorbed: String,
    pub resistance_increase: String,
    pub resistance_decrease: String,
    pub level_up: String,
    pub skill_learned: String,
    pub battle_start: String,
    pub miss: String,
    pub shop_greeting1: String,
    pub shop_regreeting1: String,
    pub shop_buy1: String,
    pub shop_sell1: String,
    pub shop_leave1: String,
    pub shop_buy_select1: String,
    pub shop_buy_number1: String,
    pub shop_purchased1: String,
    pub shop_sell_select1: String,
    pub shop_sell_number1: String,
    pub shop_sold1: String,
    pub shop_greeting2: String,
    pub shop_regreeting2: String,
    pub shop_buy2: String,
    pub shop_sell2: String,
    pub shop_leave2: String,
    pub shop_buy_select2: String,
    pub shop_buy_number2: String,
    pub shop_purchased2: String,
    pub shop_sell_select2: String,
    pub shop_sell_number2: String,
    pub shop_sold2: String,
    pub shop_greeting3: String,
    pub shop_regreeting3: String,
    pub shop_buy3: String,
    pub shop_sell3: String,
    pub shop_leave3: String,
    pub shop_buy_select3: String,
    pub shop_buy_number3: String,
    pub shop_purchased3: String,
    pub shop_sell_select3: String,
    pub shop_sell_number3: String,
    pub shop_sold3: String,
    pub inn_a_greeting_1: String,
    pub inn_a_greeting_2: String,
    pub inn_a_greeting_3: String,
    pub inn_a_accept: String,
    pub inn_a_cancel: String,
    pub inn_b_greeting_1: String,
    pub inn_b_greeting_2: String,
    pub inn_b_greeting_3: String,
    pub inn_b_accept: String,
    pub inn_b_cancel: String,
    pub possessed_items: String,
    pub equipped_items: String,
    pub gold: String,
    pub battle_fight: String,
    pub battle_auto: String,
    pub battle_escape: String,
    pub command_attack: String,
    pub command_defend: String,
    pub command_item: String,
    pub command_skill: String,
    pub menu_equipment: String,
    pub menu_save: String,
    pub menu_quit: String,
    pub new_game: String,
    pub load_game: String,
    pub exit_game: String,
    pub status: String,
    pub row: String,
    pub order: String,
    pub wait_on: String,
    pub wait_off: String,
    pub level: String,
    pub health_points: String,
    pub spirit_points: String,
    pub normal_status: String,
    pub exp_short: String,
    pub lvl_short: String,
    pub hp_short: String,
    pub sp_short: String,
    pub sp_cost: String,
    pub attack: String,
    pub defense: String,
    pub spirit: String,
    pub agility: String,
    pub weapon: String,
    pub shield: String,
    pub armor: String,
    pub helmet: String,
    pub accessory: String,
    pub save_game_message: String,
    pub load_game_message: String,
    pub file: String,
    pub exit_game_message: String,
    pub yes: String,
    pub no: String,
}

const TERMS_SECTION: u32 = 0x15;

/// Parse the vocabulary definition (`ChunkData::terms` = `0x15`) out of an LDB
/// byte slice. The section is a single struct — a bare `[id][size][data]` chunk
/// stream with no `[count]` header — so every term string is read directly by its
/// `ChunkTerms` id and CP1250-decoded; any unrecognised id (the RPG2003, Maniac,
/// or EasyRPG extension terms) is skipped by its length.
pub fn parse_terms(bytes: &[u8]) -> Result<Terms, LcfError> {
    let section = find_section(bytes, TERMS_SECTION, LcfError::MissingTerms)?;
    let mut reader = Reader::new(section);
    let mut t = Terms::default();
    while !reader.is_empty() {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let field = reader.take(size)?;
        match id {
            0x01 => t.encounter = decode_cp1250(field),
            0x02 => t.special_combat = decode_cp1250(field),
            0x03 => t.escape_success = decode_cp1250(field),
            0x04 => t.escape_failure = decode_cp1250(field),
            0x05 => t.victory = decode_cp1250(field),
            0x06 => t.defeat = decode_cp1250(field),
            0x07 => t.exp_received = decode_cp1250(field),
            0x08 => t.gold_recieved_a = decode_cp1250(field),
            0x09 => t.gold_recieved_b = decode_cp1250(field),
            0x0A => t.item_recieved = decode_cp1250(field),
            0x0B => t.attacking = decode_cp1250(field),
            0x0C => t.enemy_critical = decode_cp1250(field),
            0x0D => t.actor_critical = decode_cp1250(field),
            0x0E => t.defending = decode_cp1250(field),
            0x0F => t.observing = decode_cp1250(field),
            0x10 => t.focus = decode_cp1250(field),
            0x11 => t.autodestruction = decode_cp1250(field),
            0x12 => t.enemy_escape = decode_cp1250(field),
            0x13 => t.enemy_transform = decode_cp1250(field),
            0x14 => t.enemy_damaged = decode_cp1250(field),
            0x15 => t.enemy_undamaged = decode_cp1250(field),
            0x16 => t.actor_damaged = decode_cp1250(field),
            0x17 => t.actor_undamaged = decode_cp1250(field),
            0x18 => t.skill_failure_a = decode_cp1250(field),
            0x19 => t.skill_failure_b = decode_cp1250(field),
            0x1A => t.skill_failure_c = decode_cp1250(field),
            0x1B => t.dodge = decode_cp1250(field),
            0x1C => t.use_item = decode_cp1250(field),
            0x1D => t.hp_recovery = decode_cp1250(field),
            0x1E => t.parameter_increase = decode_cp1250(field),
            0x1F => t.parameter_decrease = decode_cp1250(field),
            0x20 => t.enemy_hp_absorbed = decode_cp1250(field),
            0x21 => t.actor_hp_absorbed = decode_cp1250(field),
            0x22 => t.resistance_increase = decode_cp1250(field),
            0x23 => t.resistance_decrease = decode_cp1250(field),
            0x24 => t.level_up = decode_cp1250(field),
            0x25 => t.skill_learned = decode_cp1250(field),
            0x26 => t.battle_start = decode_cp1250(field),
            0x27 => t.miss = decode_cp1250(field),
            0x29 => t.shop_greeting1 = decode_cp1250(field),
            0x2A => t.shop_regreeting1 = decode_cp1250(field),
            0x2B => t.shop_buy1 = decode_cp1250(field),
            0x2C => t.shop_sell1 = decode_cp1250(field),
            0x2D => t.shop_leave1 = decode_cp1250(field),
            0x2E => t.shop_buy_select1 = decode_cp1250(field),
            0x2F => t.shop_buy_number1 = decode_cp1250(field),
            0x30 => t.shop_purchased1 = decode_cp1250(field),
            0x31 => t.shop_sell_select1 = decode_cp1250(field),
            0x32 => t.shop_sell_number1 = decode_cp1250(field),
            0x33 => t.shop_sold1 = decode_cp1250(field),
            0x36 => t.shop_greeting2 = decode_cp1250(field),
            0x37 => t.shop_regreeting2 = decode_cp1250(field),
            0x38 => t.shop_buy2 = decode_cp1250(field),
            0x39 => t.shop_sell2 = decode_cp1250(field),
            0x3A => t.shop_leave2 = decode_cp1250(field),
            0x3B => t.shop_buy_select2 = decode_cp1250(field),
            0x3C => t.shop_buy_number2 = decode_cp1250(field),
            0x3D => t.shop_purchased2 = decode_cp1250(field),
            0x3E => t.shop_sell_select2 = decode_cp1250(field),
            0x3F => t.shop_sell_number2 = decode_cp1250(field),
            0x40 => t.shop_sold2 = decode_cp1250(field),
            0x43 => t.shop_greeting3 = decode_cp1250(field),
            0x44 => t.shop_regreeting3 = decode_cp1250(field),
            0x45 => t.shop_buy3 = decode_cp1250(field),
            0x46 => t.shop_sell3 = decode_cp1250(field),
            0x47 => t.shop_leave3 = decode_cp1250(field),
            0x48 => t.shop_buy_select3 = decode_cp1250(field),
            0x49 => t.shop_buy_number3 = decode_cp1250(field),
            0x4A => t.shop_purchased3 = decode_cp1250(field),
            0x4B => t.shop_sell_select3 = decode_cp1250(field),
            0x4C => t.shop_sell_number3 = decode_cp1250(field),
            0x4D => t.shop_sold3 = decode_cp1250(field),
            0x50 => t.inn_a_greeting_1 = decode_cp1250(field),
            0x51 => t.inn_a_greeting_2 = decode_cp1250(field),
            0x52 => t.inn_a_greeting_3 = decode_cp1250(field),
            0x53 => t.inn_a_accept = decode_cp1250(field),
            0x54 => t.inn_a_cancel = decode_cp1250(field),
            0x55 => t.inn_b_greeting_1 = decode_cp1250(field),
            0x56 => t.inn_b_greeting_2 = decode_cp1250(field),
            0x57 => t.inn_b_greeting_3 = decode_cp1250(field),
            0x58 => t.inn_b_accept = decode_cp1250(field),
            0x59 => t.inn_b_cancel = decode_cp1250(field),
            0x5C => t.possessed_items = decode_cp1250(field),
            0x5D => t.equipped_items = decode_cp1250(field),
            0x5F => t.gold = decode_cp1250(field),
            0x65 => t.battle_fight = decode_cp1250(field),
            0x66 => t.battle_auto = decode_cp1250(field),
            0x67 => t.battle_escape = decode_cp1250(field),
            0x68 => t.command_attack = decode_cp1250(field),
            0x69 => t.command_defend = decode_cp1250(field),
            0x6A => t.command_item = decode_cp1250(field),
            0x6B => t.command_skill = decode_cp1250(field),
            0x6C => t.menu_equipment = decode_cp1250(field),
            0x6E => t.menu_save = decode_cp1250(field),
            0x70 => t.menu_quit = decode_cp1250(field),
            0x72 => t.new_game = decode_cp1250(field),
            0x73 => t.load_game = decode_cp1250(field),
            0x75 => t.exit_game = decode_cp1250(field),
            0x76 => t.status = decode_cp1250(field),
            0x77 => t.row = decode_cp1250(field),
            0x78 => t.order = decode_cp1250(field),
            0x79 => t.wait_on = decode_cp1250(field),
            0x7A => t.wait_off = decode_cp1250(field),
            0x7B => t.level = decode_cp1250(field),
            0x7C => t.health_points = decode_cp1250(field),
            0x7D => t.spirit_points = decode_cp1250(field),
            0x7E => t.normal_status = decode_cp1250(field),
            0x7F => t.exp_short = decode_cp1250(field),
            0x80 => t.lvl_short = decode_cp1250(field),
            0x81 => t.hp_short = decode_cp1250(field),
            0x82 => t.sp_short = decode_cp1250(field),
            0x83 => t.sp_cost = decode_cp1250(field),
            0x84 => t.attack = decode_cp1250(field),
            0x85 => t.defense = decode_cp1250(field),
            0x86 => t.spirit = decode_cp1250(field),
            0x87 => t.agility = decode_cp1250(field),
            0x88 => t.weapon = decode_cp1250(field),
            0x89 => t.shield = decode_cp1250(field),
            0x8A => t.armor = decode_cp1250(field),
            0x8B => t.helmet = decode_cp1250(field),
            0x8C => t.accessory = decode_cp1250(field),
            0x92 => t.save_game_message = decode_cp1250(field),
            0x93 => t.load_game_message = decode_cp1250(field),
            0x94 => t.file = decode_cp1250(field),
            0x97 => t.exit_game_message = decode_cp1250(field),
            0x98 => t.yes = decode_cp1250(field),
            0x99 => t.no = decode_cp1250(field),
            _ => {}
        }
    }
    Ok(t)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{make_ldb, subchunk};
    use crate::{LcfError, parse_terms};

    #[test]
    fn parses_menu_battle_shop_and_currency_terms() {
        // ASCII values, so the CP1250 decode matches the source bytes byte-for-byte
        // and the test exercises the chunk-id -> field mapping, not the encoding.
        let mut section = Vec::new();
        section.extend(subchunk(0x05, b"Victory"));
        section.extend(subchunk(0x5F, b"Gold"));
        section.extend(subchunk(0x65, b"Fight"));
        section.extend(subchunk(0x68, b"Attack"));
        section.extend(subchunk(0x6A, b"Item"));
        section.extend(subchunk(0x6C, b"Equip"));
        section.extend(subchunk(0x29, b"Welcome"));
        // An unknown extension id (EasyRPG status-scene name) is skipped.
        section.extend(subchunk(0xCB, b"ignored"));

        let ldb = make_ldb(&[(0x15, section)]);
        let terms = parse_terms(&ldb).unwrap();

        assert_eq!(terms.victory, "Victory");
        assert_eq!(terms.gold, "Gold");
        assert_eq!(terms.battle_fight, "Fight");
        assert_eq!(terms.command_attack, "Attack");
        assert_eq!(terms.command_item, "Item");
        assert_eq!(terms.menu_equipment, "Equip");
        assert_eq!(terms.shop_greeting1, "Welcome");
        assert!(terms.command_skill.is_empty());
    }

    #[test]
    fn decodes_cp1250_accented_terms() {
        // 0xF5 is 'ő' in Windows-1250; the parser must decode it, not pass bytes.
        let section = subchunk(0x05, &[b'G', b'y', 0xF5, b'z']);
        let ldb = make_ldb(&[(0x15, section)]);
        let terms = parse_terms(&ldb).unwrap();
        assert_eq!(terms.victory, "Győz");
    }

    #[test]
    fn absent_section_reports_missing_terms() {
        let ldb = make_ldb(&[(0x14, vec![0])]);
        assert!(matches!(parse_terms(&ldb), Err(LcfError::MissingTerms)));
    }
}
