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
pub(super) fn command_step(
    keys: &ButtonInput<KeyCode>,
    cursor: &mut usize,
    buy: usize,
    sell: usize,
) -> Transition {
    if keys.just_pressed(KeyCode::Escape) {
        return Transition::Leave(Se::Cancel);
    }
    if confirm(keys) {
        return match *cursor {
            0 => Transition::To(Phase::Buy { cursor: buy }, Se::Decision),
            1 => Transition::To(Phase::Sell { cursor: sell }, Se::Decision),
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
    if confirm(keys) {
        let Some(item) = ids.get(*cursor).and_then(|&id| data.item(id)) else {
            return Transition::Stay(Se::Buzzer);
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

/// Zero-price rows remain visible but cannot open the quantity window.
pub(super) fn sell_step(
    keys: &ButtonInput<KeyCode>,
    data: &GameData,
    inventory: &Inventory,
    allow_buy: bool,
    cursor: &mut usize,
) -> Transition {
    let ids = logic::sell_ids(data, inventory);
    if keys.just_pressed(KeyCode::Escape) {
        return back_or_leave(allow_buy, Se::Cancel);
    }
    if confirm(keys) {
        let Some(item) = ids.get(*cursor).and_then(|&id| data.item(id)) else {
            return Transition::Stay(Se::Buzzer);
        };
        if item.price == 0 {
            return Transition::Stay(Se::Buzzer);
        }
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

/// Quantity navigation precedes this decision, including simultaneous cancel.
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
    if confirm(keys) {
        let mut traded = false;
        for _ in 0..num.count {
            if !logic::apply_trade(num.mode, num.item_id, data, inventory) {
                break;
            }
            traded = true;
        }
        if !traded {
            return Transition::Stay(Se::Buzzer);
        }
        outcome.transacted = true;
        let done = match num.mode {
            Mode::Buy => Phase::Bought {
                remaining: super::clock::CONFIRM_FRAMES,
                cursor: num.origin,
                item_id: num.item_id,
            },
            Mode::Sell => Phase::Sold {
                remaining: super::clock::CONFIRM_FRAMES,
                cursor: num.origin,
                item_id: num.item_id,
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
