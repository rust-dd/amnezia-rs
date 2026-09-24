use super::*;
use crate::gamedata::GameData;
use crate::menu::DirectionInput;
use crate::state::Inventory;
use crate::terms::Terms;
use crate::timing::GameFrames;
use crate::windowskin::selectable::List;

pub(in crate::battle) mod smoke;

#[derive(Resource)]
pub(super) struct Windows {
    generation: u64,
    last: Option<u32>,
    active: Option<Panel>,
    was_moving: bool,
    lists: [List; 8],
}

impl Default for Windows {
    fn default() -> Self {
        Self {
            generation: 0,
            last: None,
            active: None,
            was_moving: false,
            lists: Panel::ALL.map(|panel| List::new(panel.columns(), 4, true)),
        }
    }
}

impl Windows {
    pub(super) fn get(&self, panel: Panel) -> &List {
        &self.lists[panel as usize]
    }

    pub(super) fn help_index(&self, battle: &Battle) -> usize {
        self.get(if layout::base_menu(battle) == MenuLevel::Item {
            Panel::Item
        } else {
            Panel::Skill
        })
        .help_index
    }

    fn observe(
        &mut self,
        battle: &Battle,
        data: &GameData,
        inventory: &Inventory,
        terms: &Terms,
        motion: &motion::CommandWindows,
    ) {
        if self.generation != battle.generation {
            *self = Self {
                generation: battle.generation,
                ..Self::default()
            };
        }
        let active = Panel::active(battle);
        for panel in Panel::ALL {
            if layout::rectangle(panel, battle).is_none() {
                continue;
            }
            let count = if panel == Panel::Status {
                battle.members.len()
            } else {
                content::rows(panel, battle, data, inventory, terms).len()
            };
            let index = if panel == Panel::Status && active != Some(panel) {
                motion.status_cursor().unwrap_or(0)
            } else {
                panel.cursor(battle)
            };
            let list = &mut self.lists[panel as usize];
            let changed = list.index != index || list.count() != count;
            if changed {
                list.refresh(index, count);
            }
            if active == Some(panel) && (changed || self.active != active) {
                list.help_index = list.index;
            }
        }
        self.active = active;
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn tick(
    frames: Res<GameFrames>,
    input: Res<DirectionInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut battle: ResMut<Battle>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    terms: Res<Terms>,
    pause: crate::transitions::TransitionPause,
    motion: Res<motion::CommandWindows>,
    mut windows: ResMut<Windows>,
) {
    windows.observe(&battle, &data, &inventory, &terms, &motion);
    let elapsed = windows
        .last
        .replace(frames.frame)
        .map_or(0, |last| frames.frame.wrapping_sub(last));
    if pause.paused() || battle.phase == Phase::Inactive || input.rewound {
        return;
    }
    let moving = motion.moving();
    let active = (!moving && !windows.was_moving && !battle.events.blocks_action())
        .then(|| Panel::active(&battle))
        .flatten();
    if elapsed > 0 || moving {
        windows.was_moving = moving;
    }
    let mut repeats = input.slot_steps();
    for step in 0..elapsed.max(1) {
        let repeated = repeats.next().unwrap_or([false; 6]);
        let triggered =
            [KeyCode::ArrowDown, KeyCode::ArrowUp].map(|key| step == 0 && keys.just_pressed(key));
        for panel in Panel::ALL {
            let list = &mut windows.lists[panel as usize];
            let moves = list.tick(repeated, triggered, active == Some(panel), elapsed > 0);
            battle.pending_se.extend(std::iter::repeat_n(
                super::super::model::BattleSe::Cursor,
                moves as usize,
            ));
            if active == Some(panel) {
                battle.cursor = list.index;
            }
        }
    }
}

pub(super) fn observe(
    battle: Res<Battle>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    terms: Res<Terms>,
    motion: Res<motion::CommandWindows>,
    mut windows: ResMut<Windows>,
) {
    windows.observe(&battle, &data, &inventory, &terms, &motion);
}

#[cfg(test)]
mod tests;
