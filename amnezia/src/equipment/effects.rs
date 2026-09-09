use amnezia_data::ItemDef;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct EquipmentEffects {
    pub prevent_critical: bool,
    pub raise_evasion: bool,
    pub half_sp_cost: bool,
}

impl EquipmentEffects {
    pub fn from_slots(slots: [u32; 5], items: &[ItemDef]) -> Self {
        let mut effects = Self::default();
        for item in slots
            .into_iter()
            .filter_map(|id| items.iter().find(|item| item.id == id))
            .filter(|item| (2..=5).contains(&item.item_type))
        {
            effects.prevent_critical |= item.prevent_critical;
            effects.raise_evasion |= item.raise_evasion;
            effects.half_sp_cost |= item.half_sp_cost;
        }
        effects
    }

    pub fn skill_cost(self, cost: u32) -> u32 {
        if self.half_sp_cost {
            cost.div_ceil(2)
        } else {
            cost
        }
    }

    pub fn physical_hit(self, chance: i32, target_can_act: bool) -> i32 {
        chance
            - if self.raise_evasion && target_can_act {
                25
            } else {
                0
            }
    }

    pub fn critical_chance(self, chance: u32) -> u32 {
        if self.prevent_critical { 0 } else { chance }
    }
}
