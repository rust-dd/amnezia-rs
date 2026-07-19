//! The shop's per-phase input steps: the Buy/Sell/Leave command menu, the buy and
//! sell lists, and the "how many?" quantity window. Each takes one keypress and
//! returns a [`Transition`] telling [`super::flow`] to hold the phase, switch to
//! another, or leave; trades run against the live gold and inventory through
//! [`super::logic`].

use bevy::prelude::*;

use super::flow::{Se, Transition, confirm};
use super::logic;
use super::{Mode, NumberState, Phase};
use crate::gamedata::GameData;
use crate::state::Inventory;

/// The Buy/Sell/Leave command menu (shown only for a full buy+sell shop).
pub(super) fn command_step(keys: &ButtonInput<KeyCode>, cursor: &mut usize) -> Transition {
    const OPTIONS: usize = 3;
    if keys.just_pressed(KeyCode::Escape) {
        return Transition::Leave(Se::Cancel);
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        *cursor = (*cursor + 1) % OPTIONS;
        return Transition::Stay(Se::Cursor);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        *cursor = (*cursor + OPTIONS - 1) % OPTIONS;
        return Transition::Stay(Se::Cursor);
    }
    if confirm(keys) {
        return match *cursor {
            0 => Transition::To(Phase::Buy { cursor: 0 }, Se::Decision),
            1 => Transition::To(Phase::Sell { cursor: 0 }, Se::Decision),
            _ => Transition::Leave(Se::Decision),
        };
    }
    Transition::Stay(Se::None)
}

/// Choosing an item to buy: cancel backs out to the menu (or leaves a buy-only
/// shop); confirm opens the quantity window for an affordable, under-cap item and
/// buzzes an unaffordable one.
pub(super) fn buy_step(
    keys: &ButtonInput<KeyCode>,
    data: &GameData,
    inventory: &Inventory,
    items: &[u32],
    allow_sell: bool,
    cursor: &mut usize,
) -> Transition {
    let ids = logic::buyable_ids(data, items);
    if keys.just_pressed(KeyCode::Escape) {
        return back_or_leave(allow_sell, Se::Cancel);
    }
    if let Some(t) = list_move(keys, ids.len(), cursor) {
        return t;
    }
    if confirm(keys) {
        let Some(item) = ids.get(*cursor).and_then(|&id| data.item(id)) else {
            return Transition::Stay(Se::None);
        };
        let owned = inventory.count(item.id);
        if logic::can_buy(item.price, inventory.gold(), owned) {
            let num = NumberState {
                mode: Mode::Buy,
                item_id: item.id,
                count: 1,
                max: logic::buy_max(item.price, inventory.gold(), owned),
                unit_price: item.price as i32,
                origin: *cursor,
            };
            return Transition::To(Phase::Number(num), Se::Decision);
        }
        return Transition::Stay(Se::Buzzer);
    }
    Transition::Stay(Se::None)
}

/// Choosing a held item to sell: cancel backs out; confirm opens the quantity
/// window. `sellable_ids` already excludes price-0 (unsellable) items.
pub(super) fn sell_step(
    keys: &ButtonInput<KeyCode>,
    data: &GameData,
    inventory: &Inventory,
    allow_buy: bool,
    cursor: &mut usize,
) -> Transition {
    let ids = logic::sellable_ids(data, inventory);
    if keys.just_pressed(KeyCode::Escape) {
        return back_or_leave(allow_buy, Se::Cancel);
    }
    if let Some(t) = list_move(keys, ids.len(), cursor) {
        return t;
    }
    if confirm(keys) {
        let Some(item) = ids.get(*cursor).and_then(|&id| data.item(id)) else {
            return Transition::Stay(Se::None);
        };
        let num = NumberState {
            mode: Mode::Sell,
            item_id: item.id,
            count: 1,
            max: inventory.count(item.id),
            unit_price: logic::sell_price(item.price),
            origin: *cursor,
        };
        return Transition::To(Phase::Number(num), Se::Decision);
    }
    Transition::Stay(Se::None)
}

/// The "how many?" window: up/down step by 1, left/right by 10 (bounded to
/// `1..=max`); confirm trades that many at once and shows the confirmation; cancel
/// returns to the list restoring the cursor. (EasyRPG's `Window_ShopNumber` maps
/// the axes the other way — this follows the remake's requested up/down = ±1.)
pub(super) fn number_step(
    keys: &ButtonInput<KeyCode>,
    data: &GameData,
    inventory: &mut Inventory,
    outcome: &mut super::ShopOutcome,
    num: &mut NumberState,
) -> Transition {
    if keys.just_pressed(KeyCode::Escape) {
        let back = match num.mode {
            Mode::Buy => Phase::Buy { cursor: num.origin },
            Mode::Sell => Phase::Sell { cursor: num.origin },
        };
        return Transition::To(back, Se::Cancel);
    }
    let previous = num.count;
    if keys.just_pressed(KeyCode::ArrowUp) {
        num.count = (num.count + 1).min(num.max);
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        num.count = num.count.saturating_sub(1).max(1);
    } else if keys.just_pressed(KeyCode::ArrowRight) {
        num.count = (num.count + 10).min(num.max);
    } else if keys.just_pressed(KeyCode::ArrowLeft) {
        num.count = num.count.saturating_sub(10).max(1);
    }
    if num.count != previous {
        return Transition::Stay(Se::Cursor);
    }
    if confirm(keys) {
        for _ in 0..num.count {
            if !logic::apply_trade(num.mode, num.item_id, data, inventory) {
                break;
            }
        }
        outcome.transacted = true;
        let done = match num.mode {
            Mode::Buy => Phase::Bought {
                timer: super::flow::CONFIRM_SECS,
            },
            Mode::Sell => Phase::Sold {
                timer: super::flow::CONFIRM_SECS,
            },
        };
        return Transition::To(done, Se::Decision);
    }
    Transition::Stay(Se::None)
}

/// Back to the command menu (as a re-greeting) when the other side is allowed,
/// else leave — the shared cancel path for the buy and sell lists.
fn back_or_leave(other_allowed: bool, se: Se) -> Transition {
    if other_allowed {
        Transition::To(
            Phase::Command {
                cursor: 0,
                regreet: true,
            },
            se,
        )
    } else {
        Transition::Leave(se)
    }
}

/// Move a list cursor with up/down within `0..len` (clamped), returning the
/// cursor transition, or `None` when no move key was pressed.
fn list_move(keys: &ButtonInput<KeyCode>, len: usize, cursor: &mut usize) -> Option<Transition> {
    if keys.just_pressed(KeyCode::ArrowDown) {
        *cursor = (*cursor + 1).min(len.saturating_sub(1));
        return Some(Transition::Stay(Se::Cursor));
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        *cursor = cursor.saturating_sub(1);
        return Some(Transition::Stay(Se::Cursor));
    }
    None
}
