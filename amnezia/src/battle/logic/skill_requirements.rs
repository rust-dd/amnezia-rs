use amnezia_data::{AttributeDef, SkillDef};

/// Physical skill attributes require a matching weapon; magic attributes do not.
pub fn weapon_allows_skill(
    skill: &SkillDef,
    weapon_attributes: &[u32],
    attributes: &[AttributeDef],
) -> bool {
    attributes
        .iter()
        .filter(|attr| attr.attribute_type == 0 && skill.attributes.contains(&attr.id))
        .all(|attr| weapon_attributes.contains(&attr.id))
}
