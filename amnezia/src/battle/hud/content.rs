use super::*;
use crate::battle::input::{command_labels, item_choices, party_labels, skill_choices};
use crate::gamedata::GameData;
use crate::i18n;
use crate::state::Inventory;
use crate::terms::Terms;

pub(super) struct Row {
    pub text: String,
    pub enabled: bool,
}

impl Row {
    fn plain(text: String) -> Self {
        Self {
            text,
            enabled: true,
        }
    }
}

pub(super) fn rows(
    panel: Panel,
    battle: &Battle,
    data: &GameData,
    inventory: &Inventory,
    terms: &Terms,
) -> Vec<Row> {
    match panel {
        Panel::Option => party_labels(terms)
            .into_iter()
            .enumerate()
            .map(|(i, text)| Row {
                text,
                enabled: i != 2 || battle.allow_escape,
            })
            .collect(),
        Panel::Command => commands(battle, data, inventory, terms),
        Panel::Status => battle
            .members
            .iter()
            .map(|f| {
                let state = if !f.alive() {
                    "kiütve"
                } else {
                    battle
                        .states
                        .iter()
                        .filter(|s| f.states.iter().any(|(id, _)| *id == s.id))
                        .max_by_key(|s| s.priority)
                        .map_or("Jó", |s| &s.name)
                };
                Row::plain(format!(
                    "{:<12}{:<6}HP{:>4}/{:>4} SP{:>4}",
                    clip(&i18n::tr(&f.name), 12),
                    clip(&i18n::tr(state), 6),
                    f.hp.max(0),
                    f.max_hp,
                    f.sp
                ))
            })
            .collect(),
        Panel::Message => {
            let log = battle.log_tail();
            let mut lines = wrap(&log, 50);
            let keep = if battle.phase == Phase::Outcome { 3 } else { 4 };
            lines = lines
                .into_iter()
                .rev()
                .take(keep)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if battle.phase == Phase::Outcome {
                lines.push("[Enter] Tovább".into());
            }
            lines.into_iter().map(Row::plain).collect()
        }
        Panel::Help => vec![Row::plain(description(battle, data, inventory))],
    }
}

fn commands(battle: &Battle, data: &GameData, inventory: &Inventory, terms: &Terms) -> Vec<Row> {
    let Some(actor) = battle.members.get(battle.turn) else {
        return Vec::new();
    };
    match battle.menu {
        MenuLevel::Command => command_labels(terms).into_iter().map(Row::plain).collect(),
        MenuLevel::Skill => skill_choices(data, &actor.known_skills, actor.equipment_effects)
            .into_iter()
            .map(|(id, cost, _)| {
                let skill = data.skills.iter().find(|s| s.id == id).unwrap();
                Row {
                    text: format!("{:<20}-{:>3}", clip(&i18n::tr(&skill.name), 20), cost),
                    enabled: battle
                        .skill_usable_by(crate::battle::model::Source::Party(battle.turn), skill),
                }
            })
            .collect(),
        MenuLevel::Item => item_choices(data, inventory)
            .into_iter()
            .map(|(id, _)| {
                let item = data.items.iter().find(|i| i.id == id).unwrap();
                Row {
                    text: format!(
                        "{:<20}:{:>3}",
                        clip(&i18n::tr(&item.name), 20),
                        inventory.count(id)
                    ),
                    enabled: crate::battle::input::item_enabled(item),
                }
            })
            .collect(),
        MenuLevel::Target => battle
            .living_enemies()
            .iter()
            .map(|&i| Row::plain(i18n::tr(&battle.enemies[i].name)))
            .collect(),
        MenuLevel::AllyTarget => Vec::new(),
    }
}

fn description(battle: &Battle, data: &GameData, inventory: &Inventory) -> String {
    let Some(actor) = battle.members.get(battle.turn) else {
        return String::new();
    };
    let value = match battle.menu {
        MenuLevel::Skill => skill_choices(data, &actor.known_skills, actor.equipment_effects)
            .get(battle.cursor)
            .and_then(|(id, _, _)| data.skills.iter().find(|s| s.id == *id))
            .map(|s| &s.description),
        MenuLevel::Item => item_choices(data, inventory)
            .get(battle.cursor)
            .and_then(|(id, _)| data.items.iter().find(|item| item.id == *id))
            .map(|i| &i.description),
        _ => None,
    };
    value.map_or_else(String::new, |s| i18n::tr(s))
}

fn clip(text: &str, cells: usize) -> String {
    text.chars().take(cells).collect()
}

fn wrap(text: &str, cells: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut current = String::new();
        for word in line.split_whitespace() {
            if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > cells {
                lines.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::model::testkit::build_party2;

    #[test]
    fn four_digit_status_values_fit_the_native_window_without_sp_maximum() {
        let mut battle = build_party2();
        for member in &mut battle.members {
            member.hp = 9999;
            member.max_hp = 9999;
            member.sp = 9999;
        }
        let data = GameData {
            actors: vec![],
            items: vec![],
            skills: vec![],
        };
        let rows = rows(
            Panel::Status,
            &battle,
            &data,
            &Inventory::default(),
            &Terms::default(),
        );
        for row in rows {
            assert!(row.text.chars().count() <= 38, "{}", row.text);
            assert!(row.text.ends_with("SP9999"));
        }
    }
}
