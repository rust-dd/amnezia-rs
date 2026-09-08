//! The battle HUD, laid out to RM2000's `Scene_Battle_Rpg2k` geometry on the
//! 320×240 screen: every battle window lives in the bottom 80px strip (one third
//! of the height). The option and command windows are 76px wide
//! (`option_command_mov`); the status window takes the remaining 244px. The strip
//! reflows by phase — the Fight/Auto/Escape option window sits on the left with the
//! status on the right; picking Fight swaps to the status on the left with the
//! Attack/Skill/Defend/Item command window on the right; a skill/item/target list
//! opens full-width over the strip; and resolution shows a full-width message
//! window. The panels use the same `System.png` 9-slice windowskin as the dialogue
//! box and render on the dedicated order-3 [`super::systems::HudCamera`], bound with
//! [`UiTargetCamera`]. They are hidden until a fight runs and recomposed whenever
//! the [`Battle`] changes.

use super::input::{command_labels, item_choices, party_labels, skill_choices};
use super::model::{Battle, MenuLevel, Phase};
use crate::font::GameFont;
use crate::gamedata::GameData;
use crate::i18n;
use crate::state::Inventory;
use crate::terms::Terms;
use bevy::prelude::*;
use bevy::text::FontSource;

/// The window strip height as a screen-height percentage — RM2000's bottom 80px of
/// 240 (`Player::screen_height - 80`).
const STRIP_H: f32 = 80.0 / 240.0 * 100.0;
/// The option / command window width as a screen-width percentage — RM2000's
/// `option_command_mov` (76px of 320).
const NARROW_W: f32 = 76.0 / 320.0 * 100.0;
/// The status window width: the rest of the strip (`MENU_WIDTH - option_command_mov`).
const WIDE_W: f32 = 100.0 - NARROW_W;

#[derive(Component)]
struct HudRoot;

/// Which battle window a panel (and its text child) is. Drives the per-phase
/// visibility and the option↔command / status reflow in [`update_hud`].
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Panel {
    /// The party-option window: Fight / Auto / Escape (party-command phase).
    Option,
    /// The actor command window: Attack / Skill / Defend / Item, and the
    /// full-width skill / item / target lists it opens.
    Command,
    /// The party status window (name, HP, SP), marking the active chooser.
    Status,
    /// The full-width battle-log window shown during resolution and the outcome.
    Message,
}

/// Tags a panel's text child with the panel it belongs to, so [`update_hud`]
/// recomposes each from the live battle.
#[derive(Component)]
struct PanelText(Panel);

/// Spawn the hidden HUD windows and bind them to the order-3 HUD camera so they
/// composite above the effect overlay. Runs after [`super::systems::spawn_hud_camera`]
/// so the camera entity exists to target.
fn spawn_hud(
    mut commands: Commands,
    font: Res<GameFont>,
    asset_server: Res<AssetServer>,
    camera: Query<Entity, With<super::systems::HudCamera>>,
) {
    let Ok(hud_camera) = camera.single() else {
        return;
    };
    let system: Handle<Image> = asset_server.load("graphics/System/System.png");
    commands
        .spawn((
            full_screen(),
            Visibility::Hidden,
            UiTargetCamera(hud_camera),
            HudRoot,
        ))
        .with_children(|root| {
            spawn_panel(root, &system, &font, option_node(), Panel::Option);
            spawn_panel(root, &system, &font, status_node(), Panel::Status);
            spawn_panel(root, &system, &font, command_node(), Panel::Command);
            spawn_panel(root, &system, &font, message_node(), Panel::Message);
        });
}

/// Toggle the HUD by phase, reflow the strip's windows to the RM2000 positions for
/// the current phase/menu, and recompose each panel's text from the live battle.
#[allow(clippy::type_complexity)]
fn update_hud(
    battle: Res<Battle>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    terms: Res<Terms>,
    mut root: Query<&mut Visibility, (With<HudRoot>, Without<Panel>)>,
    mut panels: Query<(&Panel, &mut Visibility, &mut Node), Without<HudRoot>>,
    mut texts: Query<(&PanelText, &mut Text)>,
) {
    if !battle.is_changed() {
        return;
    }
    let active = battle.phase != Phase::Inactive;
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = vis(active);
    }
    if !active {
        return;
    }
    for (panel, mut visibility, mut node) in &mut panels {
        *visibility = vis(layout_panel(*panel, &battle, &mut node));
    }
    for (slot, mut text) in &mut texts {
        **text = match slot.0 {
            Panel::Option => compose_options(&battle, &terms),
            Panel::Command => compose_command(&battle, &data, &inventory, &terms),
            Panel::Status => compose_status(&battle),
            Panel::Message => compose_message(&battle),
        };
    }
}

/// Whether `panel` shows this frame, reflowing the status and command windows to
/// their RM2000 strip positions for the current phase/menu (see `SetCommandWindowsX`
/// / `MoveCommandWindows`): option-left+status-right while picking the party option,
/// status-left+command-right while picking an actor command, and a full-width list
/// over the strip while choosing a skill / item / target.
fn layout_panel(panel: Panel, battle: &Battle, node: &mut Node) -> bool {
    let list = matches!(
        battle.menu,
        MenuLevel::Skill | MenuLevel::Item | MenuLevel::Target | MenuLevel::AllyTarget
    );
    match panel {
        Panel::Option => battle.phase == Phase::PartyCommand,
        Panel::Message => matches!(battle.phase, Phase::Resolve | Phase::Outcome),
        Panel::Status => {
            // Right of the 76px option window while picking the party option; hard
            // left while picking a command. Hidden when a full-width list covers it.
            node.left = Val::Percent(if battle.phase == Phase::PartyCommand {
                NARROW_W
            } else {
                0.0
            });
            matches!(battle.phase, Phase::PartyCommand | Phase::Command)
                && !(battle.phase == Phase::Command && list)
        }
        Panel::Command => {
            if battle.phase != Phase::Command {
                return false;
            }
            if list {
                // A skill/item/target list opens a full-width window over the strip.
                node.left = Val::Percent(0.0);
                node.width = Val::Percent(100.0);
            } else {
                // The four-command window sits narrow on the right, beside the status.
                node.left = Val::Percent(WIDE_W);
                node.width = Val::Percent(NARROW_W);
            }
            true
        }
    }
}

/// The party-option window body: Fight / Auto / Escape, cursor-marked (RM2000
/// `Window_BattleOption`).
fn compose_options(battle: &Battle, terms: &Terms) -> String {
    let mut labels = party_labels(terms);
    if !battle.allow_escape {
        labels[2] = format!("{} ×", labels[2]);
    }
    menu_rows(labels.into_iter(), battle.cursor)
}

/// The actor command window body: the four commands, or — once a sub-menu is open —
/// the skill / item / enemy / ally list the cursor indexes into. Empty unless a
/// member is choosing.
fn compose_command(
    battle: &Battle,
    data: &GameData,
    inventory: &Inventory,
    terms: &Terms,
) -> String {
    if battle.phase != Phase::Command {
        return String::new();
    }
    let Some(actor) = battle.members.get(battle.turn) else {
        return String::new();
    };
    let rows: Vec<String> = match battle.menu {
        MenuLevel::Command => command_labels(terms).to_vec(),
        MenuLevel::Skill => or_empty(
            skill_choices(data, &actor.known_skills, actor.sp)
                .into_iter()
                .map(|(_, _, label)| label)
                .collect(),
            "(nincs képesség)",
        ),
        MenuLevel::Item => or_empty(
            item_choices(data, inventory)
                .into_iter()
                .map(|(_, label)| label)
                .collect(),
            "(nincs tárgy)",
        ),
        MenuLevel::Target => battle
            .living_enemies()
            .iter()
            .map(|&i| i18n::tr(&battle.enemies[i].name))
            .collect(),
        // The ally list carries HP so a heal target is chosen against live health.
        MenuLevel::AllyTarget => battle
            .living_members()
            .iter()
            .map(|&i| {
                let m = &battle.members[i];
                format!("{}  HP {}/{}", i18n::tr(&m.name), m.hp.max(0), m.max_hp)
            })
            .collect(),
    };
    menu_rows(rows.into_iter(), battle.cursor)
}

/// The party status window body: one row per member — name, then HP/SP or the
/// knocked-out term — marking the active chooser, mirroring RM2000
/// `Window_BattleStatus` (name at the left, HP/SP to the right).
fn compose_status(battle: &Battle) -> String {
    battle
        .members
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mark = if battle.phase == Phase::Command && i == battle.turn {
                "▶"
            } else {
                " "
            };
            let state = if f.alive() {
                format!("HP {}/{}  SP {}/{}", f.hp.max(0), f.max_hp, f.sp, f.max_sp)
            } else {
                "kiütve".to_string()
            };
            format!("{mark} {}  {state}", i18n::tr(&f.name))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The full-width message window body: the recent battle log, plus the advance
/// prompt once the fight has ended.
fn compose_message(battle: &Battle) -> String {
    let mut out = battle.log_tail();
    if battle.phase == Phase::Outcome {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("[Enter] Tovább");
    }
    out
}

/// Render menu `rows` with a `▶` cursor on the selected one.
fn menu_rows(rows: impl Iterator<Item = String>, cursor: usize) -> String {
    let mut out = String::new();
    for (i, row) in rows.enumerate() {
        out.push_str(if i == cursor { "▶ " } else { "  " });
        out.push_str(&row);
        out.push('\n');
    }
    out
}

/// `rows`, or a single `fallback` row when it is empty.
fn or_empty(rows: Vec<String>, fallback: &str) -> Vec<String> {
    if rows.is_empty() {
        vec![fallback.to_string()]
    } else {
        rows
    }
}

/// `Visible` when `show`, else `Hidden`.
fn vis(show: bool) -> Visibility {
    if show {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

/// Spawn a windowskin panel (frame + tint + text) tagged with its [`Panel`] kind,
/// starting hidden until [`update_hud`] reveals it by phase.
fn spawn_panel(
    root: &mut ChildSpawnerCommands,
    system: &Handle<Image>,
    font: &GameFont,
    node: Node,
    kind: Panel,
) {
    root.spawn((node, kind, Visibility::Hidden))
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
                text_font(font, 14.0),
                TextColor(Color::WHITE),
                PanelText(kind),
            ));
        });
}

fn text_font(font: &GameFont, size: f32) -> TextFont {
    TextFont {
        font: FontSource::Handle(font.0.clone()),
        font_size: FontSize::Px(size),
        ..default()
    }
}

fn full_screen() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}

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

/// A window in the bottom strip: absolute, flush to the bottom, one strip tall,
/// with the text inset off the windowskin frame. Callers set `left`/`width`.
fn strip_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(0.0),
        height: Val::Percent(STRIP_H),
        padding: UiRect::all(Val::Px(8.0)),
        ..default()
    }
}

/// The party-option window: bottom-left, `option_command_mov` wide.
fn option_node() -> Node {
    Node {
        left: Val::Percent(0.0),
        width: Val::Percent(NARROW_W),
        ..strip_node()
    }
}

/// The actor command window: bottom-right at command width by default; [`update_hud`]
/// widens it to full width while a list is open.
fn command_node() -> Node {
    Node {
        left: Val::Percent(WIDE_W),
        width: Val::Percent(NARROW_W),
        ..strip_node()
    }
}

/// The party status window: the wide remainder of the strip; [`update_hud`] moves it
/// left (command entry) or right (option entry).
fn status_node() -> Node {
    Node {
        left: Val::Percent(NARROW_W),
        width: Val::Percent(WIDE_W),
        ..strip_node()
    }
}

/// The battle-log window: the full-width strip, shown during resolution.
fn message_node() -> Node {
    Node {
        left: Val::Percent(0.0),
        width: Val::Percent(100.0),
        ..strip_node()
    }
}

/// Register the battle HUD: spawn its windows after the HUD camera exists, then keep
/// them in sync with the live battle.
pub fn register(app: &mut App) {
    app.add_systems(Startup, spawn_hud.after(super::systems::spawn_hud_camera))
        .add_systems(Update, update_hud);
}
