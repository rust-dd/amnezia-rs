use super::*;
use crate::battle::model::{Action, Command, Source};

pub(super) fn start(world: &mut World, case: usize) {
    let mut battle = world.resource_mut::<Battle>();
    battle.phase = Phase::Resolve;
    battle.timeline = default();
    battle.queue_at = 0;
    battle.log.clear();
    battle.messages.console.clear();
    battle.pending_se.clear();
    battle.pending_action_flashes.clear();
    battle.rng = 1;
    for member in &mut battle.members {
        member.hp = 100;
        member.max_hp = 100;
        member.sp = 100;
        member.max_sp = 100;
        member.states.clear();
        member.defending = false;
        member.stat_modifiers = [0; 4];
        member.weapon_attributes.clear();
        member.weapon_states.clear();
        member.weapon_hit = if case == 2 { 0 } else { 100 };
        member.weapon_crit = if case == 1 { 100 } else { 0 };
        member.base_critical_denominator = None;
        member.attack_animation = 0;
        member.stats.attack = 20;
        member.stats.agility = 10;
    }
    for enemy in &mut battle.enemies {
        enemy.hp = 1000;
        enemy.max_hp = 1000;
        enemy.states.clear();
        enemy.dying = None;
        enemy.fled = false;
        enemy.defending = false;
        enemy.stat_modifiers = [0; 4];
        enemy.stats.defense = 4;
        enemy.stats.agility = 10;
    }
    let kind = match case {
        3 | 4 => {
            let mut skill = if case == 3 {
                battle
                    .skills
                    .iter()
                    .find(|skill| skill.id == 1)
                    .unwrap()
                    .clone()
            } else {
                battle
                    .skills
                    .iter()
                    .find(|skill| {
                        skill.scope == 3 && skill.affect_hp && skill.affected_states.is_empty()
                    })
                    .unwrap()
                    .clone()
            };
            skill.id = 990;
            skill.scope = if case == 3 { 1 } else { 3 };
            skill.power = 10;
            skill.hit = 100;
            skill.sp_cost = 3;
            skill.physical_rate = 0;
            skill.magical_rate = 0;
            skill.variance = 0;
            skill.affect_hp = true;
            skill.affect_sp = false;
            skill.affect_stats = [false; 4];
            skill.attributes.clear();
            skill.affected_states.clear();
            skill.absorb = false;
            skill.animation_id = 0;
            skill.using_message2.clear();
            battle.skills.retain(|skill| skill.id != 990);
            battle.skills.push(skill);
            if case == 4 {
                battle.members[1].hp = 1;
            }
            Command::Skill {
                skill_id: 990,
                target: usize::from(case == 4),
            }
        }
        5 => {
            battle.members[1].hp = 0;
            battle.members[1].states = vec![(1, 0)];
            Command::Item {
                item_id: 112,
                target: 1,
            }
        }
        _ => {
            if case == 6 {
                battle.enemies[0].hp = 1;
            }
            Command::Attack { target: 0 }
        }
    };
    battle.queue = vec![Action {
        source: Source::Party(0),
        kind,
        agility: 1,
    }];
    let now = world.resource::<SceneFrames>().frame;
    let mut trace = world.resource_mut::<Trace>();
    trace.epoch = Some(now);
    trace.case = case;
    if case == 5 {
        let mut inventory = world.resource_mut::<crate::state::Inventory>();
        let held = inventory.count(112);
        inventory.remove_item(112, held);
        inventory.add_item(112, 1);
    }
}

pub(super) fn verify_state(battle: &Battle, case: usize, age: u32) {
    let hp = match case {
        0 => {
            if age < 88 {
                1000
            } else {
                990
            }
        }
        1 => {
            if age < 117 {
                1000
            } else {
                968
            }
        }
        3 => {
            if age < 105 {
                1000
            } else {
                990
            }
        }
        6 => i32::from(age < 88),
        _ => 1000,
    };
    assert_eq!(battle.enemies[0].hp, hp, "case {case}, update {age}");
    assert_eq!(
        battle.enemies[1].hp,
        if case == 3 && age >= 156 { 990 } else { 1000 }
    );
    assert_eq!(
        battle.members[0].sp,
        if matches!(case, 3 | 4) && age >= 4 {
            97
        } else {
            100
        }
    );
    assert_eq!(
        battle.members[1].hp,
        match case {
            4 =>
                if age < 63 {
                    1
                } else {
                    11
                },
            5 =>
                if age < 63 {
                    0
                } else {
                    50
                },
            _ => 100,
        }
    );
    assert_eq!(battle.members[1].states.is_empty(), case != 5 || age >= 63);
    assert_eq!(battle.enemies[0].dying.is_some(), case == 6 && age >= 88);
}

pub(super) fn lines(battle: &Battle, case: usize, at: u32) -> Vec<String> {
    if at < 4 {
        return vec![];
    }
    let source = &battle.members[0].name;
    let enemy = &battle.enemies[0].name;
    let mut lines = vec![match case {
        3 | 4 => format!(
            "{source}{}",
            battle
                .skills
                .iter()
                .find(|skill| skill.id == 990)
                .unwrap()
                .using_message1
        ),
        5 => format!(
            "{source} {}{}",
            battle
                .items
                .iter()
                .find(|item| item.id == 112)
                .unwrap()
                .name,
            battle.text.use_item
        ),
        _ => format!("{source}{}", battle.text.attacking),
    }];
    match case {
        0 if at >= 49 => lines.push(battle.text.damaged(enemy, false, 10)),
        1 => {
            if at >= 46 {
                lines.push(battle.text.enemy_critical.clone());
            }
            if at >= 78 {
                lines.push(battle.text.damaged(enemy, false, 32));
            }
        }
        2 if at >= 49 => lines.push(battle.text.skill_failed(enemy, 3)),
        3 => {
            if (66..114).contains(&at) {
                lines.push(battle.text.damaged(enemy, false, 10));
            }
            if at >= 117 {
                lines.push(battle.text.damaged(&battle.enemies[1].name, false, 10));
            }
        }
        4 if at >= 66 => lines.push(battle.text.recovered(&battle.members[1].name, true, 10)),
        5 if at >= 66 => lines.push(format!(
            "{}{}",
            battle.members[1].name,
            battle
                .states
                .iter()
                .find(|state| state.id == 1)
                .unwrap()
                .message_recovery
        )),
        6 => {
            if at >= 49 {
                lines.push(battle.text.damaged(enemy, false, 10));
            }
            if at >= 88 {
                lines.push(format!(
                    "{enemy}{}",
                    battle
                        .states
                        .iter()
                        .find(|state| state.id == 1)
                        .unwrap()
                        .message_enemy
                ));
            }
        }
        _ => {}
    }
    lines
}

pub(super) fn capture(case: usize, age: u32) -> Option<&'static str> {
    match (case, age) {
        (0, 5) => Some("action-normal-use"),
        (0, 50) => Some("action-normal-damage"),
        (0, 89) => Some("action-normal-applied"),
        (1, 47) => Some("action-critical"),
        (1, 79) => Some("action-critical-damage"),
        (1, 118) => Some("action-critical-applied"),
        (2, 5) => Some("action-miss-use"),
        (2, 50) => Some("action-miss"),
        (2, 80) => Some("action-miss-held"),
        (3, 67) => Some("action-multi-first"),
        (3, 115) => Some("action-multi-cleared"),
        (3, 118) => Some("action-multi-second"),
        (4, 5) => Some("action-heal-use"),
        (4, 64) => Some("action-heal-applied"),
        (4, 67) => Some("action-heal"),
        (5, 5) => Some("action-revive-use"),
        (5, 64) => Some("action-revive-applied"),
        (5, 67) => Some("action-revive"),
        (6, 50) => Some("action-lethal-damage"),
        (6, 89) => Some("action-collapse"),
        (6, 120) => Some("action-collapse-held"),
        _ => None,
    }
}
