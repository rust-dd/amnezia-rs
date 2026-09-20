//! The shop and inn screens: windowskin overlays the interpreter opens by
//! emitting a [`ShopRequest`]. A shop honours its RM2000 type — Buy/Sell/Leave
//! for a full shop, Buy-only or Sell-only otherwise — lets the party pick a
//! quantity to trade against the real gold and inventory (capped at 99, half
//! price to sell, price-0 items unsellable), and shows the shopkeeper's lines. An
//! inn charges gold for an overnight rest, gated on affordability, and heals the
//! party to full. Both pause the game via [`ShopOpen`] (guard wired by the main
//! session); a buffered [`ShopRequest`] is the interpreter→consumer channel.
//!
//! The screen state and its phase machine live here; the pure trade rules in
//! [`logic`], the shopkeeper text in [`messages`], the input flow in [`flow`],
//! and the panel rendering in [`view`].

use bevy::prelude::*;

mod clock;
mod flow;
mod inn_music;
mod logic;
mod messages;
mod navigation;
mod quantity;
mod scene;
pub(crate) mod smoke;
mod steps;
#[cfg(test)]
mod tests;
mod view;

/// A request from the interpreter to open a merchant screen. The main session
/// wires the interpreter to emit this instead of skipping the opcodes.
#[derive(Message, Debug, Clone)]
pub enum ShopRequest {
    /// Open a shop offering `items` (item ids). `allow_buy`/`allow_sell` come from
    /// the RM2000 shop mode (0 buy+sell, 1 buy-only, 2 sell-only) and `shop_type`
    /// picks the shopkeeper's wording. Maps from `OpenShop` (code 10720).
    OpenShop {
        items: Vec<u32>,
        allow_buy: bool,
        allow_sell: bool,
        shop_type: u32,
    },
    /// Offer an overnight rest costing `cost` gold. Maps from the `ShowInn`
    /// command (code 10730).
    ShowInn { cost: i32 },
}

/// Whether a merchant screen (shop or inn) is showing; the movement/interpreter
/// pause guard reads this. This module owns the toggle.
#[derive(Resource, Default)]
pub struct ShopOpen(pub bool);

/// Whether the player actually bought or sold (or stayed at the inn) while the
/// screen was open. Reset to `false` each time a merchant screen opens and set
/// `true` on the first successful trade or inn stay, so the interpreter can pick
/// the Transaction/NoTransaction (or Stay/NoStay) branch after the screen closes.
#[derive(Resource, Default)]
pub struct ShopOutcome {
    pub transacted: bool,
}

/// What the merchant overlay is currently showing.
#[derive(Resource, Default)]
enum Screen {
    #[default]
    Closed,
    Shop(Box<ShopState>),
    Inn {
        cost: i32,
        yes: bool,
        done: bool,
    },
}

/// A live shop: the offered items, what the RM2000 mode permits, the wording set,
/// and the current step of the buy/sell interaction.
struct ShopState {
    items: Vec<u32>,
    allow_buy: bool,
    allow_sell: bool,
    shop_type: u32,
    phase: Phase,
    scene: scene::State,
}

/// The step a shop interaction is on, mirroring EasyRPG's `Scene_Shop` modes.
enum Phase {
    /// The Buy/Sell/Leave command menu (shown only for a full buy+sell shop).
    /// `regreet` swaps the greeting for the "anything else?" line after a trade.
    Command { cursor: usize, regreet: bool },
    /// Choosing an item to buy.
    Buy { cursor: usize },
    /// Choosing a held item to sell.
    Sell { cursor: usize },
    /// The "how many?" quantity window.
    Number(NumberState),
    /// The post-trade hold retains the list selection for its return.
    Bought {
        remaining: u32,
        cursor: usize,
        item_id: u32,
    },
    Sold {
        remaining: u32,
        cursor: usize,
        item_id: u32,
    },
}

/// The quantity window's state: which item and direction, the running count, the
/// affordability/stock-bounded maximum, and the per-unit price for the total.
struct NumberState {
    mode: Mode,
    item_id: u32,
    count: u32,
    max: u32,
    unit_price: i32,
    /// The select-list cursor to restore when the player backs out.
    origin: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Buy,
    Sell,
}

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ShopRequest>()
            .init_resource::<ShopOpen>()
            .init_resource::<ShopOutcome>()
            .init_resource::<Screen>()
            .init_resource::<view::party::Cache>()
            .add_systems(Startup, (view::spawn_ui, view::inn::spawn_ui))
            .add_systems(
                Update,
                (
                    flow::open_requests,
                    flow::shop_input,
                    flow::debug_triggers,
                    view::update_ui,
                    view::party::update,
                    view::inn::update,
                )
                    .chain()
                    .after(crate::menu::MenuInput),
            );
        inn_music::register(app);
    }
}
