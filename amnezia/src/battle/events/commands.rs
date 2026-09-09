use super::{BattleEvents, EventWorld, actors};
use crate::audio::AudioRequest;
use crate::battle::BattleOutcome;
use crate::battle::model::Battle;
use crate::events::message_boxes;
use crate::interpreter::apply_control_switches;
use amnezia_data::EventCommand;
use bevy::prelude::*;

pub(super) fn step(events: &mut BattleEvents, battle: &mut Battle, world: &mut EventWorld) -> bool {
    let Some(page) = events.page else { return true };
    let Some(command) = events.pages[page].commands.get(events.ip).cloned() else {
        events.page = None;
        return true;
    };
    events.ip += 1;
    match command.code {
        0 => events.page = None,
        10 | 23311 => {}
        10110 | 10130 | 20110 => {
            let start = events.ip - 1;
            let commands = &events.pages[page].commands;
            let count = commands[start..]
                .iter()
                .take_while(|c| matches!(c.code, 10110 | 10130 | 20110))
                .count();
            let mut boxes =
                message_boxes(&commands[start..start + count], &mut world.dialogue.face);
            events.ip = start + count;
            if !boxes.is_empty() {
                for message in &mut boxes {
                    for line in &mut message.lines {
                        *line = crate::i18n::tr(line);
                    }
                }
                world.dialogue.open(boxes);
                return false;
            }
        }
        10210 => apply_control_switches(&mut world.switches, &command.params),
        10330 => actors::change_party(&command.params, battle, world),
        10440 => actors::change_skills(&command.params, battle, world),
        10460 => actors::change_hp(&command.params, battle, world),
        11410 => {
            events.wait = command.params.first().copied().unwrap_or(0).max(0) as f32 / 10.0;
            return false;
        }
        11510 => {
            world
                .audio
                .write(AudioRequest::play_bgm(&command.string, &command.params));
        }
        11550 => {
            world
                .audio
                .write(AudioRequest::play_sound(&command.string, &command.params));
        }
        13110 => enemy_hp(&command.params, battle, world),
        13260 => {
            let [animation, target, wait, ..] = command.params.as_slice() else {
                return true;
            };
            let targets = battle
                .enemies
                .iter()
                .enumerate()
                .filter(|(index, enemy)| {
                    if *target < 0 {
                        enemy.alive()
                    } else {
                        *index == *target as usize
                    }
                })
                .map(|(index, _)| battle.foe_anim_pos(index))
                .collect::<Vec<_>>();
            let plays = *animation > 0 && !targets.is_empty();
            battle.push_anim((*animation).max(0) as u32, targets);
            if *wait != 0 && plays {
                battle.begin_anim_hold();
                events.wait_animation = true;
                return false;
            }
        }
        13310 => {
            if !branch(&command.params, battle, world) {
                skip(events, &command, &[23310, 23311]);
            }
        }
        23310 => skip(events, &command, &[23311]),
        13410 => {
            battle.finish(BattleOutcome::Abort);
            events.page = None;
            events.checkpoint = false;
            return false;
        }
        code => warn!(code, "Unsupported troop event command"),
    }
    true
}

fn skip(events: &mut BattleEvents, command: &EventCommand, terminators: &[u32]) {
    let commands = &events.pages[events.page.unwrap()].commands;
    events.ip = commands
        .iter()
        .enumerate()
        .skip(events.ip)
        .find(|(_, c)| c.indent == command.indent && terminators.contains(&c.code))
        .map_or(commands.len(), |(index, _)| index + 1);
}

fn branch(params: &[i32], battle: &Battle, world: &EventWorld) -> bool {
    let [kind, id, operand_type, operand, operator, ..] = params else {
        return false;
    };
    match kind {
        0 => world.switches.get(*id as u32) == (*operand_type == 0),
        1 => {
            let a = world.variables.get(*id as u32);
            let b = if *operand_type == 0 {
                *operand
            } else {
                world.variables.get(*operand as u32)
            };
            match operator {
                0 => a == b,
                1 => a >= b,
                2 => a <= b,
                3 => a > b,
                4 => a < b,
                5 => a != b,
                _ => false,
            }
        }
        2 => battle
            .members
            .iter()
            .find(|f| f.actor_id == *id as u32)
            .is_some_and(|f| {
                f.alive() && crate::battle::logic::worst_restriction(&f.states, &battle.states) != 1
            }),
        3 => battle.enemies.get(*id as usize).is_some_and(|f| {
            f.alive() && crate::battle::logic::worst_restriction(&f.states, &battle.states) != 1
        }),
        _ => false,
    }
}

fn enemy_hp(params: &[i32], battle: &mut Battle, world: &EventWorld) {
    let [index, operation, operand_type, operand, lethal, ..] = params else {
        return;
    };
    let Some(enemy) = battle.enemies.get_mut(*index as usize) else {
        return;
    };
    if enemy.hp <= 0 {
        return;
    }
    let value = match operand_type {
        0 => *operand,
        1 => world.variables.get(*operand as u32),
        2 => (i64::from(*operand) * i64::from(enemy.hp) / 100) as i32,
        _ => return,
    };
    let delta = if *operation > 0 { -value } else { value };
    enemy.hp = enemy
        .hp
        .saturating_add(delta)
        .clamp(i32::from(*lethal == 0), enemy.max_hp);
    if enemy.hp == 0 {
        enemy.states.clear();
        battle.start_foe_death(*index as usize, false);
    }
}
