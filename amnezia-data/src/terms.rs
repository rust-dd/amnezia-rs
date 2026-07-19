//! The game's vocabulary (RM2000 `rpg::Terms`), converted into a clean RON asset
//! the game's chrome reads. These are the *real* Hungarian UI strings from the
//! original `RPG_RT.ldb` Terms section — the menu command labels, the status /
//! equipment labels, the currency term, and the battle / shop / inn message
//! terms — replacing the invented Hungarian placeholders the chrome shipped with.
//! The game routes each through `i18n::tr()`, so they localise to English exactly
//! like the rest of the Hungarian source text.
//!
//! Battle and reward terms are RM2000 name-concatenation templates, not
//! `%S`-placeholder strings: the engine forms a line by prefixing the battler name
//! or suffixing the value/term (see EasyRPG `game_message_terms.cpp`).

use serde::{Deserialize, Serialize};

/// The chrome-facing subset of the RM2000 term vocabulary. Every field is
/// `#[serde(default)]` so a `terms.ron` written before a field existed still
/// loads (the missing term reads as an empty string, and the chrome falls back to
/// its own default when a term is blank). Field names mirror liblcf's `rpg::Terms`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermsDef {
    #[serde(default)]
    pub command_attack: String,
    #[serde(default)]
    pub command_defend: String,
    #[serde(default)]
    pub command_item: String,
    #[serde(default)]
    pub command_skill: String,
    #[serde(default)]
    pub menu_equipment: String,
    #[serde(default)]
    pub menu_save: String,
    #[serde(default)]
    pub menu_quit: String,
    #[serde(default)]
    pub battle_fight: String,
    #[serde(default)]
    pub battle_auto: String,
    #[serde(default)]
    pub battle_escape: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub row: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub health_points: String,
    #[serde(default)]
    pub spirit_points: String,
    #[serde(default)]
    pub normal_status: String,
    #[serde(default)]
    pub exp_short: String,
    #[serde(default)]
    pub lvl_short: String,
    #[serde(default)]
    pub hp_short: String,
    #[serde(default)]
    pub sp_short: String,
    #[serde(default)]
    pub attack: String,
    #[serde(default)]
    pub defense: String,
    #[serde(default)]
    pub spirit: String,
    #[serde(default)]
    pub agility: String,
    #[serde(default)]
    pub weapon: String,
    #[serde(default)]
    pub shield: String,
    #[serde(default)]
    pub armor: String,
    #[serde(default)]
    pub helmet: String,
    #[serde(default)]
    pub accessory: String,
    #[serde(default)]
    pub gold: String,
    #[serde(default)]
    pub attacking: String,
    #[serde(default)]
    pub defending: String,
    #[serde(default)]
    pub dodge: String,
    #[serde(default)]
    pub miss: String,
    #[serde(default)]
    pub enemy_critical: String,
    #[serde(default)]
    pub actor_critical: String,
    #[serde(default)]
    pub enemy_damaged: String,
    #[serde(default)]
    pub actor_damaged: String,
    #[serde(default)]
    pub victory: String,
    #[serde(default)]
    pub defeat: String,
    #[serde(default)]
    pub escape_success: String,
    #[serde(default)]
    pub escape_failure: String,
    #[serde(default)]
    pub exp_received: String,
    #[serde(default)]
    pub gold_recieved_a: String,
    #[serde(default)]
    pub gold_recieved_b: String,
    #[serde(default)]
    pub item_recieved: String,
    #[serde(default)]
    pub level_up: String,
    #[serde(default)]
    pub shop_greeting1: String,
    #[serde(default)]
    pub shop_regreeting1: String,
    #[serde(default)]
    pub shop_buy1: String,
    #[serde(default)]
    pub shop_sell1: String,
    #[serde(default)]
    pub shop_leave1: String,
    #[serde(default)]
    pub shop_buy_select1: String,
    #[serde(default)]
    pub shop_sell_select1: String,
    #[serde(default)]
    pub shop_buy_number1: String,
    #[serde(default)]
    pub shop_purchased1: String,
    #[serde(default)]
    pub shop_sold1: String,
    #[serde(default)]
    pub shop_greeting2: String,
    #[serde(default)]
    pub shop_regreeting2: String,
    #[serde(default)]
    pub shop_buy2: String,
    #[serde(default)]
    pub shop_sell2: String,
    #[serde(default)]
    pub shop_leave2: String,
    #[serde(default)]
    pub shop_buy_select2: String,
    #[serde(default)]
    pub shop_sell_select2: String,
    #[serde(default)]
    pub shop_buy_number2: String,
    #[serde(default)]
    pub shop_purchased2: String,
    #[serde(default)]
    pub shop_sold2: String,
    #[serde(default)]
    pub shop_greeting3: String,
    #[serde(default)]
    pub shop_regreeting3: String,
    #[serde(default)]
    pub shop_buy3: String,
    #[serde(default)]
    pub shop_sell3: String,
    #[serde(default)]
    pub shop_leave3: String,
    #[serde(default)]
    pub shop_buy_select3: String,
    #[serde(default)]
    pub shop_sell_select3: String,
    #[serde(default)]
    pub shop_buy_number3: String,
    #[serde(default)]
    pub shop_purchased3: String,
    #[serde(default)]
    pub shop_sold3: String,
    #[serde(default)]
    pub inn_a_greeting_1: String,
    #[serde(default)]
    pub inn_a_greeting_2: String,
    #[serde(default)]
    pub inn_a_greeting_3: String,
    #[serde(default)]
    pub inn_a_accept: String,
    #[serde(default)]
    pub inn_a_cancel: String,
    #[serde(default)]
    pub inn_b_greeting_1: String,
    #[serde(default)]
    pub inn_b_greeting_2: String,
    #[serde(default)]
    pub inn_b_greeting_3: String,
    #[serde(default)]
    pub inn_b_accept: String,
    #[serde(default)]
    pub inn_b_cancel: String,
}

impl TermsDef {
    /// The three-set shop vocabulary indexed by RM2000 `shop_type` (0/1/2): each
    /// merchant style keys a parallel run of greeting / buy / sell / prompt /
    /// confirmation terms. An out-of-range type falls back to set 1.
    pub fn shop_set(&self, shop_type: u32) -> ShopTerms<'_> {
        match shop_type {
            1 => ShopTerms {
                greeting: &self.shop_greeting2,
                regreeting: &self.shop_regreeting2,
                buy: &self.shop_buy2,
                sell: &self.shop_sell2,
                leave: &self.shop_leave2,
                buy_select: &self.shop_buy_select2,
                sell_select: &self.shop_sell_select2,
                number: &self.shop_buy_number2,
                purchased: &self.shop_purchased2,
                sold: &self.shop_sold2,
            },
            2 => ShopTerms {
                greeting: &self.shop_greeting3,
                regreeting: &self.shop_regreeting3,
                buy: &self.shop_buy3,
                sell: &self.shop_sell3,
                leave: &self.shop_leave3,
                buy_select: &self.shop_buy_select3,
                sell_select: &self.shop_sell_select3,
                number: &self.shop_buy_number3,
                purchased: &self.shop_purchased3,
                sold: &self.shop_sold3,
            },
            _ => ShopTerms {
                greeting: &self.shop_greeting1,
                regreeting: &self.shop_regreeting1,
                buy: &self.shop_buy1,
                sell: &self.shop_sell1,
                leave: &self.shop_leave1,
                buy_select: &self.shop_buy_select1,
                sell_select: &self.shop_sell_select1,
                number: &self.shop_buy_number1,
                purchased: &self.shop_purchased1,
                sold: &self.shop_sold1,
            },
        }
    }
}

/// One merchant style's shop terms, borrowed from a [`TermsDef`] by
/// [`TermsDef::shop_set`]. The chrome routes each through `i18n::tr()`.
pub struct ShopTerms<'a> {
    pub greeting: &'a str,
    pub regreeting: &'a str,
    pub buy: &'a str,
    pub sell: &'a str,
    pub leave: &'a str,
    pub buy_select: &'a str,
    pub sell_select: &'a str,
    pub number: &'a str,
    pub purchased: &'a str,
    pub sold: &'a str,
}
