//! The merchant panel: the RM2000 windowskin overlay and the text it shows for
//! the current [`Screen`] phase — the shopkeeper's line, the buy/sell list or
//! quantity window, and the party's gold — plus the inn's Yes/No prompt.

use crate::font::GameFont;
use crate::gamedata::GameData;
use crate::i18n;
use crate::state::Inventory;
use bevy::prelude::*;
use bevy::text::FontSource;

use super::logic;
use super::messages::{self, GOLD_LABEL, GOLD_UNIT};
use super::{Mode, Phase, Screen, ShopState};

#[derive(Component)]
pub struct ShopPanel;

#[derive(Component)]
pub struct ShopText;

/// Reflect the screen state into the panel whenever it changes.
pub fn update_ui(
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
        Screen::Shop(state) => render_shop(state, data, inventory),
        Screen::Inn { cost, yes, done } => render_inn(*cost, *yes, *done, inventory),
    }
}

fn render_shop(state: &ShopState, data: &GameData, inventory: &Inventory) -> String {
    let vocab = messages::shop_vocab(state.shop_type);
    let gold = format!("{GOLD_LABEL}: {}", inventory.gold());
    let mut out = String::new();
    match &state.phase {
        Phase::Command { cursor, regreet } => {
            let header = if *regreet {
                vocab.regreeting
            } else {
                vocab.greeting
            };
            out.push_str(header);
            out.push_str("\n\n");
            for (i, label) in [vocab.buy, vocab.sell, vocab.leave].iter().enumerate() {
                out.push_str(cursor_mark(i == *cursor));
                out.push_str(label);
                out.push('\n');
            }
        }
        Phase::Buy { cursor } => {
            out.push_str(vocab.buy_select);
            out.push_str("\n\n");
            let ids = logic::buyable_ids(data, &state.items);
            list_rows(&mut out, &ids, *cursor, data, inventory, Mode::Buy);
        }
        Phase::Sell { cursor } => {
            out.push_str(vocab.sell_select);
            out.push_str("\n\n");
            let ids = logic::sellable_ids(data, inventory);
            list_rows(&mut out, &ids, *cursor, data, inventory, Mode::Sell);
        }
        Phase::Number(num) => {
            out.push_str(vocab.number);
            out.push_str("\n\n");
            let name = data
                .item(num.item_id)
                .map(|i| i18n::tr(&i.name))
                .unwrap_or_default();
            let total = num.unit_price * num.count as i32;
            out.push_str(&format!(
                "{name}   × {}\n\nÖsszesen: {total}{GOLD_UNIT}\n",
                num.count
            ));
        }
        Phase::Bought { .. } => {
            out.push_str(vocab.purchased);
            out.push('\n');
        }
        Phase::Sold { .. } => {
            out.push_str(vocab.sold);
            out.push('\n');
        }
    }
    out.push('\n');
    out.push_str(&gold);
    out
}

/// Append the buy or sell list rows (item name, unit price, and — when selling —
/// the owned count), marking the cursor row.
fn list_rows(
    out: &mut String,
    ids: &[u32],
    cursor: usize,
    data: &GameData,
    inventory: &Inventory,
    mode: Mode,
) {
    if ids.is_empty() {
        out.push_str("  (nincs áru)\n");
        return;
    }
    for (i, &id) in ids.iter().enumerate() {
        let Some(item) = data.item(id) else {
            continue;
        };
        let name = i18n::tr(&item.name);
        let row = match mode {
            Mode::Buy => format!("{name}   {}{GOLD_UNIT}", item.price),
            Mode::Sell => format!(
                "{name}   {}{GOLD_UNIT}   ×{}",
                logic::sell_price(item.price),
                inventory.count(id)
            ),
        };
        out.push_str(cursor_mark(i == cursor));
        out.push_str(&row);
        out.push('\n');
    }
}

fn render_inn(cost: i32, yes: bool, done: bool, inventory: &Inventory) -> String {
    let inn = messages::inn_vocab();
    let cost = cost.max(0);
    let gold = format!("{GOLD_LABEL}: {}", inventory.gold());
    if done {
        return format!("{}\n\n(-{cost}{GOLD_UNIT})\n\n{gold}", inn.rested);
    }
    let affordable = logic::inn_afford(cost, inventory.gold()).is_some();
    let accept = if affordable {
        inn.accept.to_string()
    } else {
        format!("{} {}", inn.accept, inn.broke)
    };
    format!(
        "Egy szoba {cost}{GOLD_UNIT}.\nKipihened magad?\n\n{}{accept}\n{}{}\n\n{gold}",
        cursor_mark(yes),
        cursor_mark(!yes),
        inn.cancel,
    )
}

/// The cursor prefix for a selected / unselected row.
fn cursor_mark(selected: bool) -> &'static str {
    if selected { "▶ " } else { "  " }
}

/// Spawn the initially hidden, centred merchant panel, styled with the same
/// RM2000 windowskin (`System.png`) as the dialogue box and sitting above it.
pub fn spawn_ui(mut commands: Commands, font: Res<GameFont>, asset_server: Res<AssetServer>) {
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
