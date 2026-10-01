//! Trade rules and inventory mutations, matching EasyRPG `Window_ShopBuy`/`Window_ShopSell`
//! `CheckEnable` and `Scene_Shop::UpdateNumberInput`, independent of Bevy input.

use super::Mode;
use crate::gamedata::GameData;
use crate::state::Inventory;

/// RPG_RT `GetMaxItemCount` purchase limit.
const MAX_ITEM_STACK: u32 = 99;

/// The gold left after buying at `price`, or `None` when the party cannot afford
/// it. Buying never goes into debt.
pub fn buy(price: i32, gold: i32) -> Option<i32> {
    (gold >= price).then_some(gold - price)
}

/// What an item bought for `price` sells back for: half, rounded down.
pub fn sell_price(price: u32) -> i32 {
    (price / 2) as i32
}

/// RPG_RT CheckEnable requires enough gold for one item and room under the stock cap.
pub fn can_buy(price: u32, gold: i32, owned: u32) -> bool {
    price as i32 <= gold && owned < MAX_ITEM_STACK
}

/// Maximum affordable quantity within the stock cap; free items are bounded by stock only.
pub fn buy_max(price: u32, gold: i32, owned: u32) -> u32 {
    let stock = MAX_ITEM_STACK.saturating_sub(owned);
    match (gold.max(0) as u32).checked_div(price) {
        Some(by_gold) => stock.min(by_gold),
        None => stock,
    }
}

/// Trade one unit, returning success. Quantity trades repeat this check per unit.
pub fn apply_trade(mode: Mode, id: u32, data: &GameData, inventory: &mut Inventory) -> bool {
    let Some(item) = data.item(id) else {
        return false;
    };
    match mode {
        Mode::Buy => {
            let price = item.price as i32;
            if inventory.count(id) < MAX_ITEM_STACK && buy(price, inventory.gold()).is_some() {
                inventory.remove_gold(price);
                inventory.add_item(id, 1);
                return true;
            }
            false
        }
        Mode::Sell => {
            if item.price > 0 && inventory.count(id) > 0 {
                inventory.remove_item(id, 1);
                inventory.add_gold(sell_price(item.price));
                return true;
            }
            false
        }
    }
}

/// The offered item ids that resolve to a real item, in offer order — the rows of
/// the buy list.
pub fn buyable_ids(data: &GameData, items: &[u32]) -> Vec<u32> {
    items
        .iter()
        .copied()
        .filter(|&id| data.item(id).is_some())
        .collect()
}

/// All held rows in database order, including disabled zero-price items.
pub fn sell_ids(data: &GameData, inventory: &Inventory) -> Vec<u32> {
    data.items
        .iter()
        .filter(|i| inventory.count(i.id) > 0)
        .map(|i| i.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use amnezia_data::ItemDef;

    fn item(id: u32, price: u32) -> ItemDef {
        ItemDef {
            prevent_critical: false,
            raise_evasion: false,
            half_sp_cost: false,
            actor_set: Vec::new(),
            state_chance: 0,
            id,
            name: format!("item{id}"),
            description: String::new(),
            item_type: 1,
            price,
            recover_hp: 0,
            recover_hp_rate: 0,
            recover_sp: 0,
            recover_sp_rate: 0,
            cure_states: vec![],
            state_defense: vec![],
            attribute_defense: vec![],
            scope: 0,
            only_field: false,
            ko_only: false,
            uses: 0,
            atk: 0,
            def: 0,
            spi: 0,
            agi: 0,
            two_handed: false,
            hit: 0,
            crit: 0,
            weapon_animation: 0,
        }
    }

    fn data(items: Vec<ItemDef>) -> GameData {
        GameData {
            actors: vec![],
            items,
            skills: vec![],
        }
    }

    #[test]
    fn buy_deducts_when_affordable_and_refuses_otherwise() {
        assert_eq!(buy(100, 250), Some(150));
        assert_eq!(buy(100, 100), Some(0));
        assert_eq!(buy(100, 99), None);
    }

    #[test]
    fn sell_price_is_half_rounded_down() {
        assert_eq!(sell_price(0), 0);
        assert_eq!(sell_price(1), 0);
        assert_eq!(sell_price(100), 50);
        assert_eq!(sell_price(4001), 2000);
    }

    #[test]
    fn apply_trade_round_trips_gold_and_items() {
        let data = data(vec![item(2, 1200)]);
        let mut inv = Inventory::default();
        inv.add_gold(2000);
        assert!(apply_trade(Mode::Buy, 2, &data, &mut inv));
        assert_eq!(inv.gold(), 800);
        assert_eq!(inv.count(2), 1);
        assert!(apply_trade(Mode::Sell, 2, &data, &mut inv));
        assert_eq!(inv.gold(), 1400);
        assert_eq!(inv.count(2), 0);
        assert!(!apply_trade(Mode::Sell, 2, &data, &mut inv));
        assert_eq!(inv.gold(), 1400);
        assert!(apply_trade(Mode::Buy, 2, &data, &mut inv));
        assert_eq!(inv.count(2), 1);
    }

    #[test]
    fn buy_max_respects_gold_and_the_99_cap() {
        assert_eq!(buy_max(100, 250, 0), 2);
        assert_eq!(buy_max(10, 100_000, 95), 4);
        assert_eq!(buy_max(0, 50, 0), 99);
        assert_eq!(buy_max(100, 50, 0), 0);
        assert_eq!(buy_max(100, 100_000, 99), 0);
    }

    #[test]
    fn buying_n_applies_total_and_stops_at_the_cap() {
        let data = data(vec![item(5, 10)]);
        let mut inv = Inventory::default();
        inv.add_gold(100_000);
        inv.add_item(5, 95);
        let max = buy_max(10, inv.gold(), inv.count(5));
        assert_eq!(max, 4);
        for _ in 0..max {
            assert!(apply_trade(Mode::Buy, 5, &data, &mut inv));
        }
        assert_eq!(inv.count(5), 99);
        assert_eq!(inv.gold(), 100_000 - 40);
        assert!(!apply_trade(Mode::Buy, 5, &data, &mut inv));
        assert_eq!(inv.count(5), 99);
    }

    #[test]
    fn buying_n_stops_at_affordability() {
        let data = data(vec![item(5, 10)]);
        let mut inv = Inventory::default();
        inv.add_gold(25);
        let max = buy_max(10, inv.gold(), 0);
        assert_eq!(max, 2);
        for _ in 0..max {
            assert!(apply_trade(Mode::Buy, 5, &data, &mut inv));
        }
        assert_eq!(inv.gold(), 5);
        assert!(!apply_trade(Mode::Buy, 5, &data, &mut inv));
    }

    #[test]
    fn zero_price_items_remain_in_the_sell_list_but_cannot_be_traded() {
        let data = data(vec![item(1, 0), item(2, 100)]);
        let mut inv = Inventory::default();
        inv.add_item(1, 3);
        inv.add_item(2, 2);
        assert_eq!(sell_ids(&data, &inv), [1, 2]);
        assert!(!apply_trade(Mode::Sell, 1, &data, &mut inv));
        assert_eq!(inv.count(1), 3);
    }

    #[test]
    fn buyable_ids_drop_unknown_items() {
        let data = data(vec![item(1, 10), item(2, 20)]);
        assert_eq!(buyable_ids(&data, &[1, 999, 2, 0]), vec![1, 2]);
    }
}
