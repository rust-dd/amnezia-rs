use super::{Phase, ShopState, logic};
use crate::gamedata::GameData;
use crate::menu::DirectionInput;
use crate::state::Inventory;
use crate::windowskin::selectable::List;
use bevy::prelude::*;

pub(super) struct State {
    pub buy: List,
    pub sell: List,
    pub command_cursor: usize,
    pub command_frame: u32,
    pub number_frame: u32,
    pub party_frame: u32,
    pub help_id: u32,
    pub item_id: u32,
    pub updated: bool,
    initialized: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            buy: List::new(1, 7, false),
            sell: List::new(2, 7, false),
            command_cursor: 0,
            command_frame: 0,
            number_frame: 0,
            party_frame: 0,
            help_id: 0,
            item_id: 0,
            updated: false,
            initialized: false,
        }
    }
}

impl ShopState {
    pub(super) fn set_phase(&mut self, mut phase: Phase, data: &GameData, inventory: &Inventory) {
        self.scene.help_id = 0;
        match &mut phase {
            Phase::Command { cursor, .. } => *cursor = self.scene.command_cursor,
            Phase::Buy { cursor } => self
                .scene
                .buy
                .refresh(*cursor, logic::buyable_ids(data, &self.items).len()),
            Phase::Sell { cursor } => self
                .scene
                .sell
                .refresh(*cursor, logic::sell_ids(data, inventory).len().max(1)),
            Phase::Number(number) => self.scene.item_id = number.item_id,
            Phase::Bought { item_id, .. } => self.scene.item_id = *item_id,
            Phase::Sold {
                item_id, cursor, ..
            } => {
                self.scene.item_id = *item_id;
                self.scene
                    .sell
                    .refresh(*cursor, logic::sell_ids(data, inventory).len().max(1));
            }
        }
        self.phase = phase;
    }
}

pub(super) fn update(
    state: &mut ShopState,
    input: &DirectionInput,
    keys: &ButtonInput<KeyCode>,
    ticks: u32,
    data: &GameData,
    inventory: &Inventory,
) -> u32 {
    initialize(state, data, inventory);
    state.scene.updated = true;
    let buy = logic::buyable_ids(data, &state.items);
    let sell = logic::sell_ids(data, inventory);
    state.scene.party_frame = (state.scene.party_frame + ticks % 48) % 48;
    state.scene.command_frame = (state.scene.command_frame + ticks % 21) % 21;
    let buying = matches!(state.phase, Phase::Buy { .. });
    let selling = matches!(state.phase, Phase::Sell { .. });
    if matches!(state.phase, Phase::Number(_)) {
        state.scene.number_frame = (state.scene.number_frame + ticks % 21) % 21;
    }
    let mut moves = 0;
    for (index, repeated) in input.slot_steps().enumerate() {
        let triggered =
            [KeyCode::ArrowDown, KeyCode::ArrowUp].map(|key| index == 0 && keys.just_pressed(key));
        moves += state
            .scene
            .buy
            .tick(repeated, triggered, buying, input.timed());
        moves += state
            .scene
            .sell
            .tick(repeated, triggered, selling, input.timed());
        if let Phase::Command { cursor, .. } = &mut state.phase {
            for (action, pressed) in repeated[..2].iter().enumerate() {
                if *pressed {
                    *cursor = (*cursor + if action == 0 { 1 } else { 2 }) % 3;
                    moves += 1;
                }
            }
            state.scene.command_cursor = *cursor;
        }
    }
    match &mut state.phase {
        Phase::Buy { cursor } => {
            *cursor = state.scene.buy.index;
            state.scene.help_id = buy.get(state.scene.buy.help_index).copied().unwrap_or(0);
            state.scene.item_id = buy.get(*cursor).copied().unwrap_or(0);
        }
        Phase::Sell { cursor } => {
            *cursor = state.scene.sell.index;
            state.scene.help_id = sell.get(state.scene.sell.help_index).copied().unwrap_or(0);
        }
        Phase::Number(number) => state.scene.item_id = number.item_id,
        _ => {}
    }
    moves
}

pub(super) fn initialize(state: &mut ShopState, data: &GameData, inventory: &Inventory) {
    if state.scene.initialized {
        return;
    }
    let buy = logic::buyable_ids(data, &state.items);
    let sell = logic::sell_ids(data, inventory);
    state.scene.buy.refresh(0, buy.len());
    state.scene.sell.refresh(0, sell.len().max(1));
    if let Phase::Buy { cursor } = state.phase {
        state.scene.buy.refresh(cursor, buy.len());
    }
    if let Phase::Sell { cursor } = state.phase {
        state.scene.sell.refresh(cursor, sell.len().max(1));
    }
    state.scene.initialized = true;
}
