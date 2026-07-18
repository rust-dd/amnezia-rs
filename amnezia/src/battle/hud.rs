//! The battle HUD: the three windowskin panels (command, status, log/message)
//! styled with the same `System.png` 9-slice as the dialogue box, laid along the
//! bottom of the screen. They render on a dedicated order-2 [`HudCamera`] (spawned
//! in [`super`]) so they sit ABOVE the order-1 effect overlay the backdrop,
//! battlers, and animations draw on — bound to it with [`UiTargetCamera`] since
//! the main camera owns the default UI. The panels are hidden until a fight runs
//! and their text is recomposed whenever the [`Battle`] changes.

use super::input::{COMMAND_LABELS, item_choices, skill_choices};
use super::model::{Battle, MenuLevel, Phase};
use crate::font::GameFont;
use crate::gamedata::GameData;
use crate::i18n;
use crate::state::Inventory;
use bevy::prelude::*;
use bevy::text::FontSource;

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct LogText;

#[derive(Component)]
struct CommandText;

#[derive(Component)]
struct StatusText;

/// Spawn the hidden HUD windows and bind them to the order-2 HUD camera, so they
/// composite above the effect overlay. Runs after [`super::spawn_hud_camera`], so
/// the camera entity exists to target.
fn spawn_hud(
    mut commands: Commands,
    font: Res<GameFont>,
    asset_server: Res<AssetServer>,
    camera: Query<Entity, With<super::HudCamera>>,
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
            spawn_panel(root, &system, &font, command_node(), CommandText);
            spawn_panel(root, &system, &font, status_node(), StatusText);
            spawn_panel(root, &system, &font, log_node(), LogText);
        });
}

/// Toggle the HUD by phase and, while a fight runs, recompose the three panels'
/// text from the live battle.
#[allow(clippy::type_complexity)]
fn update_hud(
    battle: Res<Battle>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    mut root: Query<&mut Visibility, With<HudRoot>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<LogText>>,
        Query<&mut Text, With<CommandText>>,
        Query<&mut Text, With<StatusText>>,
    )>,
) {
    if !battle.is_changed() {
        return;
    }
    let active = battle.phase != Phase::Inactive;
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !active {
        return;
    }
    if let Ok(mut text) = texts.p0().single_mut() {
        **text = battle.log_tail();
    }
    if let Ok(mut text) = texts.p1().single_mut() {
        **text = compose_command(&battle, &data, &inventory);
    }
    if let Ok(mut text) = texts.p2().single_mut() {
        **text = compose_status(&battle);
    }
}

/// The command panel body: the active member's menu (or a phase note when not
/// choosing), with the cursor marking the current row.
fn compose_command(battle: &Battle, data: &GameData, inventory: &Inventory) -> String {
    if battle.phase != Phase::Command {
        return match battle.phase {
            Phase::Outcome => "[Enter] Tovább".to_string(),
            _ => "...".to_string(),
        };
    }
    let Some(actor) = battle.members.get(battle.turn) else {
        return String::new();
    };
    let rows: Vec<String> = match battle.menu {
        MenuLevel::Command => COMMAND_LABELS.iter().map(|s| s.to_string()).collect(),
        MenuLevel::Skill => or_empty(
            skill_choices(data, actor.sp)
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
        MenuLevel::AllyTarget => battle
            .living_members()
            .iter()
            .map(|&i| i18n::tr(&battle.members[i].name))
            .collect(),
    };
    let mut out = format!("{} parancsa:\n", i18n::tr(&actor.name));
    for (i, row) in rows.iter().enumerate() {
        out.push_str(if i == battle.cursor { "▶ " } else { "  " });
        out.push_str(row);
        out.push('\n');
    }
    out
}

/// The status panel body: one line per member with HP/SP, marking the chooser.
fn compose_status(battle: &Battle) -> String {
    battle
        .members
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mark = if battle.phase == Phase::Command && i == battle.turn {
                "▶ "
            } else {
                "  "
            };
            let state = if f.alive() {
                format!("HP {}/{}  SP {}/{}", f.hp.max(0), f.max_hp, f.sp, f.max_sp)
            } else {
                "kiütve".to_string()
            };
            format!("{mark}{}  {state}", i18n::tr(&f.name))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `rows`, or a single `fallback` row when it is empty.
fn or_empty(rows: Vec<String>, fallback: &str) -> Vec<String> {
    if rows.is_empty() {
        vec![fallback.to_string()]
    } else {
        rows
    }
}

/// Spawn a windowskin panel (frame + tint + text) carrying text `marker`.
fn spawn_panel<M: Component>(
    root: &mut ChildSpawnerCommands,
    system: &Handle<Image>,
    font: &GameFont,
    node: Node,
    marker: M,
) {
    root.spawn(node).with_children(|panel| {
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
            text_font(font, 15.0),
            TextColor(Color::WHITE),
            marker,
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

/// The command window: bottom-left, above the log strip (RM2000 actor-command
/// window). Tall enough for the command title plus its rows.
fn command_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(4.0),
        bottom: Val::Percent(12.0),
        width: Val::Percent(48.0),
        min_height: Val::Percent(22.0),
        padding: UiRect::all(Val::Px(10.0)),
        ..default()
    }
}

/// The status window: bottom-right, above the log strip (RM2000 party status).
fn status_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        right: Val::Px(4.0),
        bottom: Val::Percent(12.0),
        width: Val::Percent(48.0),
        min_height: Val::Percent(22.0),
        padding: UiRect::all(Val::Px(10.0)),
        ..default()
    }
}

/// The log/message window: the full-width strip along the very bottom.
fn log_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(4.0),
        right: Val::Px(4.0),
        bottom: Val::Px(4.0),
        min_height: Val::Percent(12.0),
        padding: UiRect::all(Val::Px(8.0)),
        ..default()
    }
}

/// Register the battle HUD: spawn its windows after the HUD camera exists, then
/// keep them in sync with the live battle.
pub fn register(app: &mut App) {
    app.add_systems(Startup, spawn_hud.after(super::spawn_hud_camera))
        .add_systems(Update, update_hud);
}
