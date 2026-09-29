use super::*;
use crate::equipment::EquipmentEffects;

impl Battle {
    pub(in crate::battle::resolve) fn battler_equipment_effects(
        &self,
        source: Source,
    ) -> EquipmentEffects {
        match source {
            Source::Party(i) => self
                .members
                .get(i)
                .map(|f| f.equipment_effects)
                .unwrap_or_default(),
            Source::Enemy(_) => EquipmentEffects::default(),
        }
    }
}
