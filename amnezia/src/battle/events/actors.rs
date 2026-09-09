use super::EventWorld;
use crate::battle::logic::actor_hp_sp_at;
use crate::battle::model::{Battle, Fighter, Source};
use crate::interpreter::{actor_targets, operate_value};

pub(super) fn health(id: u32, battle: &Battle, world: &EventWorld) -> Option<(i32, i32)> {
    if let Some(fighter) = battle.members.iter().find(|f| f.actor_id == id) {
        return Some((fighter.hp, fighter.max_hp));
    }
    let actor = world.data.actor(id)?;
    let (max, _) = actor_hp_sp_at(
        &actor.curves,
        world.progression.level(actor),
        actor.hp,
        actor.sp,
    );
    Some((
        world.vitals.get_stored(id).map_or(max as i32, |v| v.0),
        max as i32,
    ))
}

pub(super) fn change_hp(params: &[i32], battle: &mut Battle, world: &mut EventWorld) {
    let [mode, actor, operation, operand_type, operand, lethal, ..] = params else {
        return;
    };
    let delta = operate_value(*operation, *operand_type, *operand, &world.variables);
    for id in actor_targets(*mode, *actor, &world.variables, &world.party) {
        if let Some(index) = battle.members.iter().position(|f| f.actor_id == id) {
            let fighter = &mut battle.members[index];
            if fighter.alive() {
                fighter.hp = fighter
                    .hp
                    .saturating_add(delta)
                    .clamp(i32::from(*lethal == 0), fighter.max_hp);
                if !fighter.alive() {
                    battle.mark_knocked_out(crate::battle::model::Source::Party(index));
                }
            }
        } else if let Some((hp, max)) = health(id, battle, world)
            && hp > 0
            && let Some(actor) = world.data.actor(id)
        {
            let (_, max_sp) = actor_hp_sp_at(
                &actor.curves,
                world.progression.level(actor),
                actor.hp,
                actor.sp,
            );
            let sp = world.vitals.get_stored(id).map_or(max_sp as i32, |v| v.1);
            world.vitals.set(
                id,
                hp.saturating_add(delta).clamp(i32::from(*lethal == 0), max),
                sp,
            );
        }
    }
}

pub(super) fn change_skills(params: &[i32], battle: &mut Battle, world: &mut EventWorld) {
    let [mode, actor, remove, operand_type, operand, ..] = params else {
        return;
    };
    let skill = operate_value(0, *operand_type, *operand, &world.variables);
    if skill <= 0 || !world.data.skills.iter().any(|s| s.id == skill as u32) {
        return;
    }
    for id in actor_targets(*mode, *actor, &world.variables, &world.party) {
        if let Some(actor) = world.data.actor(id) {
            world
                .progression
                .change_skill(actor, skill as u32, *remove == 0);
            if let Some(fighter) = battle.members.iter_mut().find(|f| f.actor_id == id) {
                fighter.known_skills = world.progression.known_skill_ids(actor);
            }
        }
    }
}

pub(super) fn change_party(params: &[i32], battle: &mut Battle, world: &mut EventWorld) {
    let [operation, operand_type, operand, ..] = params else {
        return;
    };
    let id = operate_value(0, *operand_type, *operand, &world.variables).max(0) as u32;
    let Some(actor) = world.data.actor(id) else {
        return;
    };
    match operation {
        0 => {
            world.party.add(id);
            if world.party.has(id) && !battle.members.iter().any(|f| f.actor_id == id) {
                battle.members.push(Fighter::build(
                    actor,
                    world.equipment.slots(actor),
                    &world.data.items,
                    &world.vitals,
                    &world.progression,
                ));
            }
        }
        1 => {
            world.party.remove(id);
            if let Some(index) = battle.members.iter().position(|f| f.actor_id == id) {
                let fighter = battle.members.remove(index);
                world.vitals.set(id, fighter.hp, fighter.sp);
                world
                    .vitals
                    .set_states(id, fighter.states.into_iter().map(|s| s.0).collect());
                let acted = battle.queue[..battle.queue_at]
                    .iter()
                    .filter(|a| !matches!(a.source, Source::Party(i) if i == index))
                    .count();
                battle
                    .queue
                    .retain(|a| !matches!(a.source, Source::Party(i) if i == index));
                for action in &mut battle.queue {
                    if let Source::Party(i) = &mut action.source
                        && *i > index
                    {
                        *i -= 1;
                    }
                }
                battle.queue_at = acted;
                battle.turn = battle.turn.min(battle.members.len().saturating_sub(1));
            }
        }
        _ => {}
    }
}
