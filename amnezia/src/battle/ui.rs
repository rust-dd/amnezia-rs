//! The front-view battle UI: a full-screen backdrop with placeholder enemy
//! battlers in front, and three windowskin panels (log, command, status) styled
//! with the same `System.png` 9-slice as the dialogue box. Everything is a hidden
//! overlay toggled by the battle phase; the enemy nodes are (re)built per fight to
//! match the live troop. Monster battler graphics are not resolvable from the
//! converted data (Hungarian monster names vs English `Monster/*.png` files with
//! no mapping field), so enemies render as coloured, named placeholders.

use super::input::{COMMAND_LABELS, item_choices, skill_choices};
use super::model::{Battle, MenuLevel, Phase};
use crate::assets::resolve_png;
use crate::font::GameFont;
use crate::gamedata::GameData;
use crate::state::Inventory;
use bevy::prelude::*;
use bevy::text::FontSource;

/// The RM2000 battle field is 320×240 px; enemy positions map into it as
/// percentages of the screen.
const FIELD_W: f32 = 320.0;
const FIELD_H: f32 = 240.0;

#[derive(Component)]
pub struct BattleRoot;

#[derive(Component)]
struct BattleBg;

#[derive(Component)]
struct LogText;

#[derive(Component)]
struct CommandText;

#[derive(Component)]
struct StatusText;

/// A placeholder enemy battler node, indexed into [`Battle::enemies`].
#[derive(Component)]
struct EnemyNode {
    index: usize,
}

/// Spawn the hidden battle overlay: a backdrop, then three windowskin panels
/// stacked above it. Enemy nodes are added per fight by [`sync_enemies`].
pub fn spawn_ui(mut commands: Commands, font: Res<GameFont>, asset_server: Res<AssetServer>) {
    let system: Handle<Image> = asset_server.load("graphics/System/System.png");
    commands
        .spawn((
            full_screen(),
            Visibility::Hidden,
            GlobalZIndex(118),
            BattleRoot,
        ))
        .with_children(|root| {
            root.spawn((full_screen(), ImageNode::default(), ZIndex(0), BattleBg));
            spawn_panel(root, &system, &font, log_node(), LogText);
            spawn_panel(root, &system, &font, command_node(), CommandText);
            spawn_panel(root, &system, &font, status_node(), StatusText);
        });
}

/// Rebuild the enemy battler nodes when a new fight starts (or clear them when it
/// ends), keyed on the battle's per-fight generation stamp.
fn sync_enemies(
    mut commands: Commands,
    battle: Res<Battle>,
    font: Res<GameFont>,
    roots: Query<Entity, With<BattleRoot>>,
    nodes: Query<Entity, With<EnemyNode>>,
    mut synced: Local<u64>,
) {
    if !battle.is_changed() || battle.generation == *synced {
        return;
    }
    *synced = battle.generation;
    for entity in &nodes {
        commands.entity(entity).despawn();
    }
    let Ok(root) = roots.single() else {
        return;
    };
    commands.entity(root).with_children(|root| {
        for (index, foe) in battle.enemies.iter().enumerate() {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(foe.x as f32 / FIELD_W * 100.0),
                    top: Val::Percent(foe.y as f32 / FIELD_H * 100.0),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(enemy_color(index, true, false)),
                Text::new(String::new()),
                text_font(&font, 13.0),
                TextColor(Color::WHITE),
                ZIndex(1),
                EnemyNode { index },
            ));
        }
    });
}

/// Reflect the live battle into the overlay: toggle it, set the backdrop, refresh
/// the three panels, and update each enemy node's label and tint.
#[allow(clippy::type_complexity)]
fn update_ui(
    battle: Res<Battle>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    asset_server: Res<AssetServer>,
    mut root: Query<&mut Visibility, With<BattleRoot>>,
    mut backdrop: Query<&mut ImageNode, With<BattleBg>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<LogText>>,
        Query<&mut Text, With<CommandText>>,
        Query<&mut Text, With<StatusText>>,
        Query<(&EnemyNode, &mut Text, &mut BackgroundColor)>,
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
    if let Ok(mut image) = backdrop.single_mut() {
        image.image = asset_server.load(resolve_png("Backdrop", &battle.background));
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
    for (node, mut text, mut color) in &mut texts.p3() {
        if let Some(foe) = battle.enemies.get(node.index) {
            **text = format!("{}\n{}/{}", foe.name, foe.hp.max(0), foe.max_hp);
            *color = BackgroundColor(enemy_color(
                node.index,
                foe.alive(),
                targeted(&battle, node.index),
            ));
        }
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
            .map(|&i| battle.enemies[i].name.clone())
            .collect(),
    };
    let mut out = format!("{} parancsa:\n", actor.name);
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
            format!("{mark}{}  {state}", f.name)
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

/// Whether the target cursor currently rests on enemy `index`.
fn targeted(battle: &Battle, index: usize) -> bool {
    if battle.phase != Phase::Command || battle.menu != MenuLevel::Target {
        return false;
    }
    let living = battle.living_enemies();
    living.get(battle.cursor.min(living.len().saturating_sub(1))) == Some(&index)
}

/// Enemy placeholder tint: dim when dead, gold when targeted, else a per-slot hue.
fn enemy_color(index: usize, alive: bool, targeted: bool) -> Color {
    if !alive {
        return Color::srgba(0.1, 0.1, 0.1, 0.4);
    }
    if targeted {
        return Color::srgba(0.95, 0.75, 0.2, 0.92);
    }
    const HUES: [(f32, f32, f32); 4] = [
        (0.6, 0.2, 0.25),
        (0.2, 0.35, 0.55),
        (0.3, 0.45, 0.25),
        (0.5, 0.3, 0.5),
    ];
    let (r, g, b) = HUES[index % HUES.len()];
    Color::srgba(r, g, b, 0.85)
}

/// Spawn a windowskin panel (frame + tint + text) carrying text `marker`.
fn spawn_panel<M: Component>(
    root: &mut ChildSpawnerCommands,
    system: &Handle<Image>,
    font: &GameFont,
    node: Node,
    marker: M,
) {
    root.spawn((node, ZIndex(2))).with_children(|panel| {
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
    inset_node(0.0)
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

fn log_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(8.0),
        right: Val::Px(8.0),
        top: Val::Px(8.0),
        min_height: Val::Px(40.0),
        padding: UiRect::all(Val::Px(10.0)),
        ..default()
    }
}

fn command_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(8.0),
        bottom: Val::Px(8.0),
        width: Val::Percent(50.0),
        min_height: Val::Px(120.0),
        padding: UiRect::all(Val::Px(12.0)),
        ..default()
    }
}

fn status_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        right: Val::Px(8.0),
        bottom: Val::Px(8.0),
        width: Val::Percent(44.0),
        min_height: Val::Px(120.0),
        padding: UiRect::all(Val::Px(12.0)),
        ..default()
    }
}

/// Register the battle UI systems on the given app.
pub fn register(app: &mut App) {
    app.add_systems(Startup, spawn_ui)
        .add_systems(Update, (sync_enemies, update_ui));
}
