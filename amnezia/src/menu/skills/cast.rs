use super::*;
use crate::battle::logic::{self, Stats};
use crate::interpreter::EventRng;
use amnezia_data::ActorDef;

#[cfg(test)]
mod tests;

struct CastContext<'a> {
    data: &'a GameData,
    progression: &'a Progression,
    equipment: &'a Equipment,
}

impl CastContext<'_> {
    fn stats(&self, actor: &ActorDef, vitals: &Vitals) -> Stats {
        let [attack, defense, spirit, agility] = derive::stats_with_slots(
            actor,
            self.progression.level(actor),
            &self.data.items,
            self.equipment.slots(actor),
        )
        .map(|value| value.clamp(1, 999));
        let active = vitals
            .states(actor.id)
            .into_iter()
            .map(|id| (id, 0))
            .collect::<Vec<_>>();
        logic::state_stats(
            Stats {
                attack,
                defense,
                spirit,
                agility,
            },
            &active,
            crate::conditions::definitions(),
        )
    }

    fn amount(
        &self,
        caster: &ActorDef,
        target: &ActorDef,
        skill: &SkillDef,
        vitals: &Vitals,
        rng: &mut EventRng,
    ) -> i32 {
        let base = logic::skill_effect(
            skill,
            &self.stats(caster, vitals),
            &self.stats(target, vitals),
            false,
        );
        let guards = logic::equipment_resist_slots(self.equipment.slots(target), &self.data.items);
        let base = logic::attribute_damage(
            base,
            &skill.attributes,
            crate::gamedata::attribute_definitions(),
            |id| {
                let rank = id
                    .checked_sub(1)
                    .and_then(|index| target.attribute_ranks.get(index as usize))
                    .copied()
                    .unwrap_or(2);
                rank.saturating_add(u8::from(guards.contains(&id))).min(4)
            },
        );
        if skill.variance > 0 && base > 0 {
            logic::variance_adjust(base, skill.variance as i32, rng.next_u64())
        } else {
            base
        }
    }
}

/// Field recovery ignores hit rolls and pays once only if at least one target benefits.
#[allow(clippy::too_many_arguments)]
pub(in crate::menu) fn apply_field_skill(
    caster: usize,
    target: usize,
    skill_id: u32,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &mut Vitals,
    equipment: &Equipment,
    rng: &mut EventRng,
) -> bool {
    let roster = party.snapshot();
    let Some(&caster_id) = roster.get(caster) else {
        return false;
    };
    let Some(caster_def) = data.actor(caster_id) else {
        return false;
    };
    let Some(skill) = data.skills.iter().find(|skill| skill.id == skill_id) else {
        return false;
    };
    if !can_use(caster, skill, data, party, progression, vitals, equipment) {
        return false;
    }
    let full = derive::max_hp_sp(caster_def, progression.level(caster_def));
    let (caster_hp, caster_sp) = vitals.get_stored(caster_id).unwrap_or(full);
    let cost = cost(caster_def, skill, data, equipment) as i32;
    let targets = match skill.scope {
        2 => vec![caster_id],
        4 => roster,
        _ => match roster.get(target) {
            Some(&id) => vec![id],
            None => return false,
        },
    };
    let context = CastContext {
        data,
        progression,
        equipment,
    };
    let mut changed = false;
    for id in targets {
        let Some(def) = data.actor(id) else { continue };
        let full = derive::max_hp_sp(def, progression.level(def));
        let (hp, sp) = vitals.get_stored(id).unwrap_or(full);
        if hp <= 0 && !skill.affected_states.contains(&1) {
            continue;
        }
        let amount = context.amount(caster_def, def, skill, vitals, rng);
        let states = vitals.states(id);
        for state in &skill.affected_states {
            vitals.change_condition(id, *state, false, full);
        }
        let revived = hp == 0 && skill.affected_states.contains(&1);
        let hp_gain = if revived && !skill.affect_hp {
            (i64::from(full.0) * i64::from(amount) / 100).min(i64::from(i32::MAX)) as i32
        } else if skill.affect_hp {
            amount
        } else {
            0
        };
        let new_hp = hp
            .saturating_add(hp_gain)
            .max(i32::from(revived))
            .min(full.0);
        let new_sp = sp
            .saturating_add(if skill.affect_sp { amount } else { 0 })
            .min(full.1);
        vitals.set(id, new_hp, new_sp);
        changed |= (hp, sp) != (new_hp, new_sp) || states != vitals.states(id);
    }
    if changed {
        let (hp, sp) = vitals
            .get_stored(caster_id)
            .unwrap_or((caster_hp, caster_sp));
        vitals.set(caster_id, hp, sp - cost);
    }
    changed
}
