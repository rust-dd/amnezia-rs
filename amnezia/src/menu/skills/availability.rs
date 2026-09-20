use super::*;
use crate::battle::logic;
use amnezia_data::ActorDef;

pub(in crate::menu) fn cost(
    actor: &ActorDef,
    skill: &SkillDef,
    data: &GameData,
    equipment: &Equipment,
) -> u32 {
    EquipmentEffects::from_slots(equipment.slots(actor), &data.items).skill_cost(skill.sp_cost)
}

pub(in crate::menu) fn can_use(
    member: usize,
    skill: &SkillDef,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
    equipment: &Equipment,
) -> bool {
    let roster = party.snapshot();
    let Some(actor) = roster.get(member).and_then(|id| data.actor(*id)) else {
        return false;
    };
    if !field_usable(skill) {
        return false;
    }
    let full = derive::max_hp_sp(actor, progression.level(actor));
    let (hp, sp) = vitals.get_stored(actor.id).unwrap_or(full);
    if hp <= 0 || sp < cost(actor, skill, data, equipment) as i32 {
        return false;
    }
    let active = vitals
        .states(actor.id)
        .into_iter()
        .map(|id| (id, 0))
        .collect::<Vec<_>>();
    let weapon_attributes = equipment
        .slots(actor)
        .into_iter()
        .filter_map(|id| data.item(id))
        .filter(|item| item.item_type == 1)
        .flat_map(|item| item.attribute_defense.iter().copied())
        .collect::<Vec<_>>();
    logic::states_allow_skill(&active, crate::conditions::definitions(), skill)
        && logic::weapon_allows_skill(
            skill,
            &weapon_attributes,
            crate::gamedata::attribute_definitions(),
        )
}
