//! Shops and inns consume [`ShopRequest`] and pause the world through [`ShopOpen`].
//! Trade rules live in [`logic`], input in [`flow`] and rendering in [`view`].

use bevy::prelude::*;

mod clock;
mod fades;
mod flow;
pub(crate) mod inn;
mod logic;
mod messages;
mod quantity;
mod scene;
pub(crate) mod smoke;
mod steps;
#[cfg(test)]
mod tests;
mod view;

pub(crate) use fades::Flow as SceneFlow;

/// Interpreter request to open a shop or inn.
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
    ShowInn {
        cost: i32,
        inn_type: u32,
        foreground: bool,
    },
}

/// Whether a merchant screen (shop or inn) is showing; the movement/interpreter
/// pause guard reads this. This module owns the toggle.
#[derive(Resource, Default)]
pub struct ShopOpen(pub bool);

/// Latched after a successful trade/stay so the interpreter can select its result branch.
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
#[derive(Debug)]
enum Phase {
    /// The Buy/Sell/Leave command menu (shown only for a full buy+sell shop).
    /// `regreet` swaps the greeting for the "anything else?" line after a trade.
    Command {
        cursor: usize,
        regreet: bool,
    },
    Buy {
        cursor: usize,
    },
    Sell {
        cursor: usize,
    },
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

#[derive(Debug)]
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
            .add_systems(Startup, view::spawn_ui)
            .add_systems(
                Update,
                (
                    flow::open_requests,
                    flow::shop_input,
                    flow::debug_triggers,
                    view::update_ui,
                    view::party::update,
                )
                    .chain()
                    .in_set(ShopUpdate)
                    .after(crate::interpreter::InterpreterStep)
                    .after(crate::interpreter::scenes::Commit)
                    .after(crate::menu::MenuInput),
            );
        fades::register(app);
        inn::register(app);
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ShopUpdate;

pub(crate) fn playtest_state(world: &World) -> String {
    match world.get_resource::<Screen>() {
        Some(Screen::Shop(state)) => format!("{:?}", state.phase),
        _ => String::new(),
    }
}

pub(crate) fn reset_session(world: &mut World) {
    world.insert_resource(Screen::default());
    world.insert_resource(SceneFlow::default());
    world.insert_resource(inn::State::default());
}
