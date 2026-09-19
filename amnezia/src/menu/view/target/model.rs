use crate::equipment::{Equipment, EquipmentEffects};
use crate::gamedata::GameData;
use crate::menu::MenuScreen;
use crate::state::{Inventory, Party};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Key {
    Item(u32),
    Skill(usize, u32),
}

pub(super) struct Selection {
    pub key: Key,
    pub name: String,
    pub value: u32,
    pub selected: usize,
    pub whole_party: bool,
}

impl Selection {
    pub fn new(
        screen: MenuScreen,
        data: &GameData,
        party: &Party,
        inventory: &Inventory,
        equipment: &Equipment,
    ) -> Option<Self> {
        match screen {
            MenuScreen::ItemTarget { item_id, cursor } => {
                let item = data.item(item_id)?;
                Some(Self {
                    key: Key::Item(item_id),
                    name: crate::i18n::tr(&item.name),
                    value: inventory.count(item_id),
                    selected: cursor,
                    whole_party: item.scope == 1,
                })
            }
            MenuScreen::SkillTarget {
                member,
                skill_id,
                cursor,
            } => {
                let skill = data.skills.iter().find(|skill| skill.id == skill_id)?;
                let roster = party.snapshot();
                let actor = data.actor(*roster.get(member)?)?;
                let effects = EquipmentEffects::from_slots(equipment.slots(actor), &data.items);
                Some(Self {
                    key: Key::Skill(member, skill_id),
                    name: crate::i18n::tr(&skill.name),
                    value: effects.skill_cost(skill.sp_cost),
                    selected: if skill.scope == 2 { member } else { cursor },
                    whole_party: skill.scope == 4,
                })
            }
            _ => None,
        }
    }

    pub fn cursor(&self, members: usize) -> Option<(u32, u32)> {
        if members == 0 {
            None
        } else if self.whole_party {
            Some((8, (members as u32 * 58).saturating_sub(10)))
        } else {
            Some((8 + self.selected.min(members - 1) as u32 * 58, 48))
        }
    }
}

#[derive(Resource, Default)]
pub(in crate::menu) struct Clock {
    last: Option<u32>,
    key: Option<Key>,
    pub phase: u32,
}

impl Clock {
    pub(super) fn advance(&mut self, now: u32, key: Option<Key>, paused: bool) {
        let elapsed = self
            .last
            .replace(now)
            .map_or(0, |last| now.wrapping_sub(last));
        if key.is_none() || self.key != key {
            self.phase = 0;
        } else if !paused {
            self.phase = (self.phase + elapsed % 21) % 21;
        }
        self.key = key;
    }
}

#[cfg(test)]
mod tests;
