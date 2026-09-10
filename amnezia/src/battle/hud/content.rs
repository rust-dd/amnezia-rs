use super::*;
use crate::battle::input::{command_labels, item_choices, party_labels, skill_choices};
use crate::battle::outcome_text;
use crate::battle::outcome_text::wrap;
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
        Panel::Status => Vec::new(),
        Panel::Message => {
            if battle.phase == Phase::Outcome {
                let mut lines = outcome_text::page(battle);
                lines.push("[Enter] Tovább".into());
                return lines.into_iter().map(Row::plain).collect();
            }
            let log = battle.log_tail();
            let mut lines = wrap(&log, 50);
            lines = lines
                .into_iter()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
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
        MenuLevel::Command => command_labels(terms, data.actor(actor.actor_id))
            .into_iter()
            .map(Row::plain)
            .collect(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::model::testkit::build_party2;

    #[test]
    fn command_window_tracks_the_active_original_actor_including_trays_blank_row() {
        let data = GameData {
            actors: crate::assets::load_ron(&format!("{}/actors.ron", crate::assets::asset_root())),
            items: vec![],
            skills: vec![],
        };
        let mut battle = build_party2();
        battle.menu = MenuLevel::Command;
        let terms = Terms::default();
        for (id, expected) in [
            (1, "Pengetánc"),
            (2, "Varázsdal"),
            (3, "Tigrisharc"),
            (4, "Kombó"),
            (5, ""),
            (6, "Shin-Ra-Ta"),
            (7, "Gyógyítás"),
            (8, "Draco"),
            (9, "Pusztítás"),
            (10, "Tigrisharc"),
        ] {
            battle.turn = id as usize % 2;
            battle.members[battle.turn].actor_id = id;
            let rows = commands(&battle, &data, &Inventory::default(), &terms);
            assert_eq!(rows.len(), 4);
            assert_eq!(rows[1].text, expected, "actor {id}");
            assert!(rows[1].enabled);
        }
        let mut actor = data.actors[0].clone();
        actor.rename_skill = false;
        assert_eq!(command_labels(&terms, Some(&actor))[1], "Képesség");
    }
}
