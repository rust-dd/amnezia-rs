use crate::{EventCommand, LcfError, Reader};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TroopPage {
    pub condition: TroopPageCondition,
    pub commands: Vec<EventCommand>,
}

/// RPG Maker 2000 battle-page conditions. Enemy indices are zero-based troop slots;
/// actor and switch identifiers are one-based database IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TroopPageCondition {
    pub flags: u32,
    pub switch_a_id: u32,
    pub switch_b_id: u32,
    pub variable_id: u32,
    pub variable_value: i32,
    pub turn_a: u32,
    pub turn_b: u32,
    pub fatigue_min: u32,
    pub fatigue_max: u32,
    pub enemy_index: u32,
    pub enemy_hp_min: u32,
    pub enemy_hp_max: u32,
    pub actor_id: u32,
    pub actor_hp_min: u32,
    pub actor_hp_max: u32,
}

impl Default for TroopPageCondition {
    fn default() -> Self {
        Self {
            flags: 0,
            switch_a_id: 1,
            switch_b_id: 1,
            variable_id: 1,
            variable_value: 0,
            turn_a: 0,
            turn_b: 0,
            fatigue_min: 0,
            fatigue_max: 100,
            enemy_index: 0,
            enemy_hp_min: 0,
            enemy_hp_max: 100,
            actor_id: 1,
            actor_hp_min: 0,
            actor_hp_max: 100,
        }
    }
}

pub(super) fn parse_pages(data: &[u8]) -> Result<Vec<TroopPage>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut pages = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _page_id = reader.varint()?;
        let mut page = TroopPage::default();
        loop {
            let id = reader.varint()?;
            if id == 0 {
                break;
            }
            let size = reader.varint()? as usize;
            let field = reader.take(size)?;
            match id {
                0x02 => page.condition = parse_condition(field)?,
                0x0C => page.commands = crate::map::parse_commands(field)?,
                _ => {}
            }
        }
        pages.push(page);
    }
    Ok(pages)
}

fn parse_condition(data: &[u8]) -> Result<TroopPageCondition, LcfError> {
    let mut reader = Reader::new(data);
    let mut condition = TroopPageCondition::default();
    loop {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let field = reader.take(size)?;
        if id == 0x01 {
            condition.flags = field
                .iter()
                .take(4)
                .enumerate()
                .fold(0, |flags, (byte, &value)| {
                    flags | (u32::from(value) << (byte * 8))
                });
            continue;
        }
        if !(0x02..=0x0F).contains(&id) {
            continue;
        }
        let value = Reader::new(field).varint()?;
        match id {
            0x02 => condition.switch_a_id = value,
            0x03 => condition.switch_b_id = value,
            0x04 => condition.variable_id = value,
            0x05 => condition.variable_value = value as i32,
            0x06 => condition.turn_a = value,
            0x07 => condition.turn_b = value,
            0x08 => condition.fatigue_min = value,
            0x09 => condition.fatigue_max = value,
            0x0A => condition.enemy_index = value,
            0x0B => condition.enemy_hp_min = value,
            0x0C => condition.enemy_hp_max = value,
            0x0D => condition.actor_id = value,
            0x0E => condition.actor_hp_min = value,
            0x0F => condition.actor_hp_max = value,
            _ => unreachable!(),
        }
    }
    Ok(condition)
}
