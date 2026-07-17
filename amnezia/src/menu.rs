//! The in-game menu: a windowskin overlay toggled with Escape that browses the
//! party's status, the held items, and the known skills across three tabs. It is
//! a read-only viewer in v1 — the left/right arrows switch tab and up/down move a
//! row cursor through long lists; no actions are taken. The movement/interpreter
//! pause guard that freezes the world while it is open (keyed on [`MenuOpen`]) is
//! wired by the main session; this module owns the toggle and the UI.

use crate::battle::BattleActive;
use crate::font::GameFont;
use crate::gamedata::GameData;
use crate::shop::ShopOpen;
use crate::state::{Inventory, Party};
use bevy::prelude::*;
use bevy::text::FontSource;

/// Whether the menu overlay is showing. The pause guard reads this; this module
/// owns the toggle (Escape).
#[derive(Resource, Default)]
pub struct MenuOpen(pub bool);

/// Which tab is shown (`0` party, `1` items, `2` skills) and where the row
/// cursor sits within it.
#[derive(Resource, Default)]
struct MenuState {
    tab: usize,
    cursor: usize,
}

/// The tab titles, in order; the index into this is [`MenuState::tab`].
const TABS: [&str; 3] = ["Party", "Items", "Skills"];

/// How many content rows fit in the panel before it scrolls with the cursor.
const VISIBLE_ROWS: usize = 12;

#[derive(Component)]
struct MenuPanel;

#[derive(Component)]
struct MenuText;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuOpen>()
            .init_resource::<MenuState>()
            .add_systems(Startup, spawn_ui)
            .add_systems(Update, (menu_input, update_ui));
    }
}

/// Spawn the initially hidden, near-fullscreen menu panel, styled with the same
/// RM2000 windowskin (`System.png`) as the dialogue box and sitting above it.
fn spawn_ui(mut commands: Commands, font: Res<GameFont>, asset_server: Res<AssetServer>) {
    let system: Handle<Image> = asset_server.load("graphics/System/System.png");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                right: Val::Px(24.0),
                top: Val::Px(24.0),
                bottom: Val::Px(24.0),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(100),
            MenuPanel,
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
                MenuText,
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

/// Toggle the menu on Escape; while open, switch tab with left/right and move
/// the row cursor with up/down (clamped to the active tab's rows).
#[allow(clippy::too_many_arguments)]
fn menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<GameData>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    shop: Res<ShopOpen>,
    battle: Res<BattleActive>,
    mut open: ResMut<MenuOpen>,
    mut state: ResMut<MenuState>,
) {
    // A shop or battle owns Escape while it's up, so the menu can't open over it.
    if !open.0 && (shop.0 || battle.0) {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        open.0 = !open.0;
        if open.0 {
            state.cursor = 0;
        }
        return;
    }
    if !open.0 {
        return;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        state.tab = (state.tab + 1) % TABS.len();
        state.cursor = 0;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        state.tab = (state.tab + TABS.len() - 1) % TABS.len();
        state.cursor = 0;
    }
    let max = tab_rows(state.tab, &data, &party, &inventory)
        .len()
        .saturating_sub(1);
    if keys.just_pressed(KeyCode::ArrowDown) {
        state.cursor = (state.cursor + 1).min(max);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        state.cursor = state.cursor.saturating_sub(1);
    }
}

/// Reflect the menu state into the panel: show or hide it, and recompose the
/// header and the visible slice of the active tab whenever either changes.
fn update_ui(
    open: Res<MenuOpen>,
    state: Res<MenuState>,
    data: Res<GameData>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    mut panels: Query<&mut Visibility, With<MenuPanel>>,
    mut texts: Query<&mut Text, With<MenuText>>,
) {
    if !open.is_changed() && !state.is_changed() {
        return;
    }
    if let Ok(mut visibility) = panels.single_mut() {
        *visibility = if open.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if open.0
        && let Ok(mut text) = texts.single_mut()
    {
        let rows = tab_rows(state.tab, &data, &party, &inventory);
        **text = compose(state.tab, state.cursor, &rows);
    }
}

/// The content rows of the active `tab` (no header, no cursor markers), built
/// fresh from the live party, inventory, and database.
fn tab_rows(tab: usize, data: &GameData, party: &Party, inventory: &Inventory) -> Vec<String> {
    match tab {
        0 => party_rows(data, party),
        1 => item_rows(data, inventory),
        _ => skill_rows(data),
    }
}

fn party_rows(data: &GameData, party: &Party) -> Vec<String> {
    party
        .snapshot()
        .iter()
        .map(|&id| match data.actor(id) {
            Some(a) => {
                format!(
                    "{} — {} — Lv{}   HP {}   SP {}",
                    a.name, a.title, a.level, a.hp, a.sp
                )
            }
            None => format!("#{id} (unknown)"),
        })
        .collect()
}

fn item_rows(data: &GameData, inventory: &Inventory) -> Vec<String> {
    let mut rows: Vec<String> = data
        .items
        .iter()
        .filter(|i| inventory.count(i.id) > 0)
        .map(|i| format!("{} ×{}", i.name, inventory.count(i.id)))
        .collect();
    if rows.is_empty() {
        rows.push("(no items)".to_string());
    }
    rows.push(String::new());
    rows.push(format!("Gold: {}", inventory.gold()));
    rows
}

fn skill_rows(data: &GameData) -> Vec<String> {
    data.skills
        .iter()
        .map(|s| format!("{}   (SP {})", s.name, s.sp_cost))
        .collect()
}

/// Render the tab header plus the visible window of `rows` around `cursor`,
/// marking the cursor row.
fn compose(tab: usize, cursor: usize, rows: &[String]) -> String {
    let header = TABS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if i == tab {
                format!("[{name}]")
            } else {
                format!(" {name} ")
            }
        })
        .collect::<Vec<_>>()
        .join("   ");
    let start = viewport_start(cursor, rows.len());
    let mut out = format!("{header}\n\n");
    for (i, row) in rows.iter().enumerate().skip(start).take(VISIBLE_ROWS) {
        out.push_str(if i == cursor { "▶ " } else { "  " });
        out.push_str(row);
        out.push('\n');
    }
    out
}

/// The first row index of the scroll window that keeps `cursor` visible within
/// [`VISIBLE_ROWS`], never scrolling past the end.
fn viewport_start(cursor: usize, len: usize) -> usize {
    if len <= VISIBLE_ROWS {
        0
    } else {
        cursor
            .saturating_sub(VISIBLE_ROWS - 1)
            .min(len - VISIBLE_ROWS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amnezia_data::{ActorDef, ItemDef, SkillDef};

    fn data() -> GameData {
        GameData {
            actors: vec![ActorDef {
                id: 1,
                name: "Ron".into(),
                title: "Zsoldos".into(),
                level: 2,
                max_level: 50,
                hp: 63,
                sp: 37,
            }],
            items: vec![ItemDef {
                id: 5,
                name: "Gyógyfű".into(),
                description: String::new(),
                item_type: 6,
                price: 100,
            }],
            skills: vec![SkillDef {
                id: 1,
                name: "X-Csapás".into(),
                description: String::new(),
                sp_cost: 20,
                power: 50,
                hit: 0,
            }],
        }
    }

    #[test]
    fn party_row_shows_name_title_level_and_stats() {
        let rows = party_rows(&data(), &Party::default());
        assert_eq!(
            rows,
            vec!["Ron — Zsoldos — Lv2   HP 63   SP 37".to_string()]
        );
    }

    #[test]
    fn item_rows_list_only_held_items_and_always_end_with_gold() {
        let mut inv = Inventory::default();
        inv.add_gold(250);
        let empty = item_rows(&data(), &inv);
        assert_eq!(empty.first().unwrap(), "(no items)");
        assert_eq!(empty.last().unwrap(), "Gold: 250");
        inv.add_item(5, 3);
        let held = item_rows(&data(), &inv);
        assert_eq!(held.first().unwrap(), "Gyógyfű ×3");
    }

    #[test]
    fn viewport_scrolls_to_keep_cursor_visible() {
        assert_eq!(viewport_start(0, 5), 0);
        assert_eq!(viewport_start(3, 40), 0);
        assert_eq!(viewport_start(VISIBLE_ROWS, 40), 1);
        assert_eq!(viewport_start(39, 40), 40 - VISIBLE_ROWS);
    }
}
