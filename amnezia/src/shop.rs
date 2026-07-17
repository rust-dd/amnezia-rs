//! The shop and inn screens: windowskin overlays the interpreter opens by
//! emitting a [`ShopRequest`]. The shop lets the party buy the offered items and
//! sell what it holds (at half price) against the real gold and inventory; the
//! inn charges gold for an overnight rest. Both pause the game via [`ShopOpen`]
//! (guard wired by the main session). A buffered [`ShopRequest`] message is the
//! interpreter→consumer channel, matching the audio module's pattern.

use crate::font::GameFont;
use crate::gamedata::GameData;
use crate::state::Inventory;
use crate::vitals::Vitals;
use bevy::prelude::*;
use bevy::text::FontSource;

/// A request from the interpreter to open a merchant screen. The main session
/// wires the interpreter to emit this instead of skipping the opcodes.
#[derive(Message, Debug, Clone)]
pub enum ShopRequest {
    /// Open a shop offering `items` (item ids) to buy, plus the party's goods to
    /// sell. Maps from the `OpenShop` command (code 10720).
    OpenShop { items: Vec<u32> },
    /// Offer an overnight rest costing `cost` gold. Maps from the `ShowInn`
    /// command (code 10730).
    ShowInn { cost: i32 },
}

/// Whether a merchant screen (shop or inn) is showing; the movement/interpreter
/// pause guard reads this. This module owns the toggle.
#[derive(Resource, Default)]
pub struct ShopOpen(pub bool);

/// What the merchant overlay is currently showing.
#[derive(Resource, Default)]
enum Screen {
    #[default]
    Closed,
    Shop {
        items: Vec<u32>,
        mode: Mode,
        cursor: usize,
    },
    Inn {
        cost: i32,
        yes: bool,
        done: bool,
    },
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Buy,
    Sell,
}

#[derive(Component)]
struct ShopPanel;

#[derive(Component)]
struct ShopText;

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ShopRequest>()
            .init_resource::<ShopOpen>()
            .init_resource::<Screen>()
            .add_systems(Startup, spawn_ui)
            .add_systems(
                Update,
                (open_requests, shop_input, debug_triggers, update_ui),
            );
    }
}

/// The gold left after buying at `price`, or `None` when the party cannot afford
/// it. Buying never goes into debt.
pub fn buy(price: i32, gold: i32) -> Option<i32> {
    (gold >= price).then_some(gold - price)
}

/// What an item bought for `price` sells back for: half, rounded down.
pub fn sell_price(price: u32) -> i32 {
    (price / 2) as i32
}

/// Open (or replace) the merchant screen when a [`ShopRequest`] arrives.
fn open_requests(
    mut requests: MessageReader<ShopRequest>,
    mut screen: ResMut<Screen>,
    mut open: ResMut<ShopOpen>,
) {
    for request in requests.read() {
        *screen = match request {
            ShopRequest::OpenShop { items } => Screen::Shop {
                items: items.clone(),
                mode: Mode::Buy,
                cursor: 0,
            },
            ShopRequest::ShowInn { cost } => Screen::Inn {
                cost: *cost,
                yes: true,
                done: false,
            },
        };
        open.0 = true;
    }
}

/// Drive the open screen from the keyboard: move the cursor, toggle buy/sell,
/// transact against the live inventory, and close on Escape.
fn shop_input(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<GameData>,
    mut inventory: ResMut<Inventory>,
    mut vitals: ResMut<Vitals>,
    mut screen: ResMut<Screen>,
    mut open: ResMut<ShopOpen>,
) {
    if matches!(*screen, Screen::Closed) || !any_menu_key(&keys) {
        return;
    }
    let mut current = std::mem::take(&mut *screen);
    let keep = match &mut current {
        Screen::Closed => false,
        Screen::Shop {
            items,
            mode,
            cursor,
        } => shop_step(&keys, &data, &mut inventory, items, mode, cursor),
        Screen::Inn { cost, yes, done } => {
            inn_step(&keys, &mut inventory, &mut vitals, *cost, yes, done)
        }
    };
    if keep {
        *screen = current;
    } else {
        *screen = Screen::Closed;
    }
    open.0 = keep;
}

/// Handle one keypress on the shop. Returns `false` when the player leaves.
fn shop_step(
    keys: &ButtonInput<KeyCode>,
    data: &GameData,
    inventory: &mut Inventory,
    items: &[u32],
    mode: &mut Mode,
    cursor: &mut usize,
) -> bool {
    if keys.just_pressed(KeyCode::Escape) {
        return false;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::ArrowRight) {
        *mode = if *mode == Mode::Buy {
            Mode::Sell
        } else {
            Mode::Buy
        };
        *cursor = 0;
    }
    let entries = shop_entries(*mode, data, inventory, items);
    if keys.just_pressed(KeyCode::ArrowDown) {
        *cursor = (*cursor + 1).min(entries.len().saturating_sub(1));
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        *cursor = cursor.saturating_sub(1);
    }
    if confirm(keys)
        && let Some(&(id, _)) = entries.get(*cursor)
    {
        apply_trade(*mode, id, data, inventory);
    }
    true
}

/// Handle one keypress on the inn's Yes/No prompt. Returns `false` when the
/// player leaves; on a confirmed Yes it deducts the cost, fully heals the party,
/// and shows the rest message (staying open until dismissed).
fn inn_step(
    keys: &ButtonInput<KeyCode>,
    inventory: &mut Inventory,
    vitals: &mut Vitals,
    cost: i32,
    yes: &mut bool,
    done: &mut bool,
) -> bool {
    if keys.just_pressed(KeyCode::Escape) {
        return false;
    }
    if *done {
        return !confirm(keys);
    }
    if keys.just_pressed(KeyCode::ArrowLeft)
        || keys.just_pressed(KeyCode::ArrowRight)
        || keys.just_pressed(KeyCode::ArrowUp)
        || keys.just_pressed(KeyCode::ArrowDown)
    {
        *yes = !*yes;
    }
    if confirm(keys) {
        if !*yes {
            return false;
        }
        inventory.remove_gold(cost.max(0));
        vitals.heal_all();
        *done = true;
    }
    true
}

/// Apply a buy or sell of item `id` against the live gold and inventory.
fn apply_trade(mode: Mode, id: u32, data: &GameData, inventory: &mut Inventory) {
    let Some(item) = data.item(id) else {
        return;
    };
    match mode {
        Mode::Buy => {
            let price = item.price as i32;
            if buy(price, inventory.gold()).is_some() {
                inventory.remove_gold(price);
                inventory.add_item(id, 1);
            }
        }
        Mode::Sell => {
            if inventory.count(id) > 0 {
                inventory.remove_item(id, 1);
                inventory.add_gold(sell_price(item.price));
            }
        }
    }
}

/// The `(item id, display label)` rows for the shop in `mode`: the offered items
/// when buying, the held sellable items when selling.
fn shop_entries(
    mode: Mode,
    data: &GameData,
    inventory: &Inventory,
    items: &[u32],
) -> Vec<(u32, String)> {
    match mode {
        Mode::Buy => items
            .iter()
            .filter_map(|&id| {
                data.item(id)
                    .map(|i| (id, format!("{}   {}g", i.name, i.price)))
            })
            .collect(),
        Mode::Sell => data
            .items
            .iter()
            .filter(|i| inventory.count(i.id) > 0)
            .map(|i| {
                (
                    i.id,
                    format!(
                        "{}   {}g   ×{}",
                        i.name,
                        sell_price(i.price),
                        inventory.count(i.id)
                    ),
                )
            })
            .collect(),
    }
}

/// Debug-only triggers so the shop/inn UI can be exercised before the
/// interpreter emits [`ShopRequest`]. Remove once opcodes 10720/10730 are wired.
fn debug_triggers(keys: Res<ButtonInput<KeyCode>>, mut requests: MessageWriter<ShopRequest>) {
    if keys.just_pressed(KeyCode::F7) {
        requests.write(ShopRequest::OpenShop {
            items: vec![1, 2, 3, 4],
        });
    }
    if keys.just_pressed(KeyCode::F8) {
        requests.write(ShopRequest::ShowInn { cost: 10 });
    }
}

/// Whether any key the merchant screens react to was just pressed, so the input
/// system only wakes (and marks the screen changed) on real input.
fn any_menu_key(keys: &ButtonInput<KeyCode>) -> bool {
    const RELEVANT: [KeyCode; 7] = [
        KeyCode::Escape,
        KeyCode::Enter,
        KeyCode::Space,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
    ];
    RELEVANT.iter().any(|k| keys.just_pressed(*k))
}

/// The action key: Space or Enter, as used by the dialogue and choice boxes.
fn confirm(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter)
}

/// Reflect the screen state into the panel whenever it changes.
fn update_ui(
    screen: Res<Screen>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    mut panels: Query<&mut Visibility, With<ShopPanel>>,
    mut texts: Query<&mut Text, With<ShopText>>,
) {
    if !screen.is_changed() {
        return;
    }
    let showing = !matches!(*screen, Screen::Closed);
    if let Ok(mut visibility) = panels.single_mut() {
        *visibility = if showing {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if showing && let Ok(mut text) = texts.single_mut() {
        **text = render(&screen, &data, &inventory);
    }
}

/// Compose the panel text for the current screen.
fn render(screen: &Screen, data: &GameData, inventory: &Inventory) -> String {
    match screen {
        Screen::Closed => String::new(),
        Screen::Shop {
            items,
            mode,
            cursor,
        } => render_shop(*mode, *cursor, data, inventory, items),
        Screen::Inn { cost, yes, done } => render_inn(*cost, *yes, *done),
    }
}

fn render_shop(
    mode: Mode,
    cursor: usize,
    data: &GameData,
    inventory: &Inventory,
    items: &[u32],
) -> String {
    let tab = if mode == Mode::Buy {
        "[Buy]   Sell "
    } else {
        " Buy   [Sell]"
    };
    let mut out = format!("SHOP     {tab}     Gold: {}\n\n", inventory.gold());
    let entries = shop_entries(mode, data, inventory, items);
    if entries.is_empty() {
        out.push_str("  (nothing to trade)\n");
    }
    for (i, (_, label)) in entries.iter().enumerate() {
        out.push_str(if i == cursor { "▶ " } else { "  " });
        out.push_str(label);
        out.push('\n');
    }
    out.push_str("\n[←/→] Buy/Sell    [Enter] Trade    [Esc] Leave");
    out
}

fn render_inn(cost: i32, yes: bool, done: bool) -> String {
    if done {
        return format!("INN\n\nYou rested well. (-{cost}g)\n\n[Enter] Leave");
    }
    let (y, n) = if yes {
        ("▶ Yes", "  No")
    } else {
        ("  Yes", "▶ No")
    };
    format!("INN\n\nRest for {cost} gold?\n\n{y}\n{n}\n\n[←/→] Choose   [Enter] OK   [Esc] Cancel")
}

/// Spawn the initially hidden, centred merchant panel, styled with the same
/// RM2000 windowskin (`System.png`) as the dialogue box and sitting above it.
fn spawn_ui(mut commands: Commands, font: Res<GameFont>, asset_server: Res<AssetServer>) {
    let system: Handle<Image> = asset_server.load("graphics/System/System.png");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(40.0),
                right: Val::Px(40.0),
                top: Val::Px(40.0),
                bottom: Val::Px(40.0),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(110),
            ShopPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                inset_node(0.0),
                ImageNode {
                    image: system.clone(),
                    rect: Some(Rect::new(32.0, 0.0, 64.0, 32.0)),
                    image_mode: NodeImageMode::Sliced(TextureSlicer {
                        border: BorderRect::all(8.0),
                        center_scale_mode: SliceScaleMode::Stretch,
                        sides_scale_mode: SliceScaleMode::Stretch,
                        max_corner_scale: 1.0,
                    }),
                    ..default()
                },
            ));
            panel.spawn((
                inset_node(4.0),
                ImageNode {
                    image: system.clone(),
                    rect: Some(Rect::new(0.0, 0.0, 32.0, 32.0)),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
            panel.spawn((
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(font.0.clone()),
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                ShopText,
            ));
        });
}

/// An absolutely-positioned node inset by `px` on every side of its parent.
fn inset_node(px: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(px),
        right: Val::Px(px),
        top: Val::Px(px),
        bottom: Val::Px(px),
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        use amnezia_data::ItemDef;
        let data = GameData {
            actors: vec![],
            items: vec![ItemDef {
                id: 2,
                name: "Karpenge".into(),
                description: String::new(),
                item_type: 1,
                price: 1200,
            }],
            skills: vec![],
        };
        let mut inv = Inventory::default();
        inv.add_gold(2000);
        apply_trade(Mode::Buy, 2, &data, &mut inv);
        assert_eq!(inv.gold(), 800);
        assert_eq!(inv.count(2), 1);
        apply_trade(Mode::Sell, 2, &data, &mut inv);
        assert_eq!(inv.gold(), 1400); // 800 + 1200/2
        assert_eq!(inv.count(2), 0);
        apply_trade(Mode::Buy, 2, &data, &mut inv); // 1400 < ... affordable, buy again
        assert_eq!(inv.count(2), 1);
    }
}
