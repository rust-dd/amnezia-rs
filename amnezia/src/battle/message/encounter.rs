use super::*;
use crate::battle::{Battle, Phase};
use std::collections::VecDeque;

#[derive(Default)]
enum Stage {
    #[default]
    Begin,
    Enemies,
    FirstStrike,
    Clear,
}

#[derive(Default)]
pub(super) struct Encounter {
    stage: Stage,
    lines: VecDeque<String>,
}

pub(in crate::battle) fn advance(battle: &mut Battle, controls: Controls) -> bool {
    let messages = &mut battle.messages;
    messages.console.update();
    while messages.wait.ready(controls) {
        let encounter = &mut messages.encounter;
        match encounter.stage {
            Stage::Begin => {
                messages.console.clear();
                encounter.lines = battle
                    .enemies
                    .iter()
                    .filter(|enemy| enemy.alive())
                    .flat_map(|enemy| {
                        format!(
                            "{}{}",
                            crate::i18n::tr(&enemy.name),
                            crate::i18n::tr(&battle.text.encounter)
                        )
                        .split('\n')
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                    })
                    .collect();
                if !encounter.lines.is_empty() {
                    messages.wait.set(4, 4);
                }
                encounter.stage = Stage::Enemies;
            }
            Stage::Enemies if encounter.lines.is_empty() => {
                encounter.stage = Stage::FirstStrike;
            }
            Stage::Enemies if messages.console.full() => {
                messages.console.clear();
                messages.wait.set(4, 4);
            }
            Stage::Enemies => {
                let line = encounter.lines.pop_front().unwrap();
                messages.console.push(&line);
                battle.log.push(line);
                if encounter.lines.is_empty() || messages.console.full() {
                    messages.wait.set(30, 70);
                } else {
                    messages.wait.set(8, 8);
                }
            }
            Stage::FirstStrike => {
                messages.console.clear();
                if battle.first_strike {
                    let line = crate::i18n::tr(&battle.text.special_combat);
                    messages.console.push(&line);
                    battle.log.push(line);
                    messages.wait.set(30, 70);
                }
                encounter.stage = Stage::Clear;
            }
            Stage::Clear => {
                messages.console.clear();
                battle.phase = Phase::PartyCommand;
                return true;
            }
        }
    }
    false
}
