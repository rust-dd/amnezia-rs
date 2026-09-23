use super::{Battle, BattleFlow, BattleOutcome, Phase};
use crate::dialogue::{Dialogue, MessageOptions, MessagePosition, MessageTransparent};
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::Inventory;
use crate::vitals::Vitals;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub(crate) mod smoke;
#[cfg(test)]
mod tests;

pub(super) fn apply_victory_rewards(
    mut battle: ResMut<Battle>,
    data: Res<GameData>,
    mut inventory: ResMut<Inventory>,
    mut progression: ResMut<Progression>,
) {
    if battle.phase != Phase::Outcome
        || battle.outcome != Some(BattleOutcome::Victory)
        || battle.rewarded
    {
        return;
    }
    battle.rewarded = true;
    inventory.add_gold(battle.reward_gold as i32);
    for id in battle.reward_items.clone() {
        inventory.add_item(id, 1);
        if let Some(item) = data.items.iter().find(|item| item.id == id) {
            let line = format!(
                "{}{}",
                crate::i18n::tr(&item.name),
                crate::i18n::tr(&battle.text.item_recieved)
            );
            battle.outcome_message.reward(&line, "\\.");
            battle.log.push(line);
        }
    }
    let mut actors = Vec::<Vec<String>>::new();
    for fighter in &battle.members {
        if !fighter.alive() {
            continue;
        }
        if let Some(def) = data.actor(fighter.actor_id) {
            let before = progression.level(def);
            let mut known = progression.known_skill_ids(def);
            progression.add(def, battle.reward_exp);
            let after = progression.level(def);
            if after > before {
                let mut lines = vec![format!(
                    "{} {} {}{}",
                    fighter.name,
                    crate::i18n::tr(&battle.text.level),
                    after,
                    crate::i18n::tr(&battle.text.level_up)
                )];
                for learn in &def.learnings {
                    if learn.level > before
                        && learn.level <= after
                        && !known.contains(&learn.skill_id)
                        && let Some(skill) = data.skills.iter().find(|s| s.id == learn.skill_id)
                    {
                        known.push(learn.skill_id);
                        lines.push(format!(
                            "{}{}",
                            crate::i18n::tr(&skill.name),
                            crate::i18n::tr(&battle.text.skill_learned)
                        ));
                    }
                }
                actors.push(lines);
            }
        }
    }
    for lines in actors {
        battle.log.extend(lines.iter().cloned());
        battle.outcome_message.actor(lines);
    }
}

#[derive(SystemParam)]
pub(super) struct Presentation<'w> {
    dialogue: ResMut<'w, Dialogue>,
    options: Option<ResMut<'w, MessageOptions>>,
    position: Option<ResMut<'w, MessagePosition>>,
    transparent: Option<ResMut<'w, MessageTransparent>>,
    pause: Option<Res<'w, crate::timing::SceneWait>>,
}

pub(super) fn outcome_input(
    mut battle: ResMut<Battle>,
    mut flow: ResMut<BattleFlow>,
    mut vitals: ResMut<Vitals>,
    mut presentation: Presentation,
) {
    if battle.phase != Phase::Outcome
        || flow.busy()
        || presentation.pause.as_ref().is_some_and(|pause| pause.0)
    {
        return;
    }
    let outcome = battle.outcome.unwrap_or(BattleOutcome::Escape);
    if matches!(outcome, BattleOutcome::Victory | BattleOutcome::Defeat) {
        if presentation.dialogue.busy() {
            return;
        }
        if !battle.outcome_message.started {
            if outcome == BattleOutcome::Defeat {
                if let Some(options) = presentation.options.as_deref_mut() {
                    options.fixed = true;
                }
                if let Some(position) = presentation.position.as_deref_mut() {
                    *position = MessagePosition::Bottom;
                }
                if let Some(transparent) = presentation.transparent.as_deref_mut() {
                    transparent.0 = false;
                }
            }
            battle.outcome_message.open(&mut presentation.dialogue);
            return;
        }
    }
    if outcome == BattleOutcome::Abort {
        presentation.dialogue.close();
    }
    for fighter in &battle.members {
        vitals.set(fighter.actor_id, fighter.hp.max(0), fighter.sp);
        let states = fighter
            .states
            .iter()
            .filter(|(id, _)| {
                battle
                    .states
                    .iter()
                    .any(|state| state.id == *id && state.persistence == 1)
            })
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        vitals.set_states(fighter.actor_id, states);
    }
    flow.leave(outcome);
}
