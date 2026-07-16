//! Dialogue: the message-box UI, plus the action-key interaction that starts an
//! event's interpreter run when the player presses it facing that event. The
//! interpreter opens boxes via [`Dialogue::open`]; this module renders them and
//! advances/closes them on the action key.

use crate::events::MessageBox;
use crate::font::GameFont;
use crate::interpreter::RunningEvent;
use crate::player::{facing_tile, Player};
use crate::state::{active_page, Inventory, Party, Switches, Variables};
use crate::teleport::Fade;
use crate::world::MapEvents;
use bevy::prelude::*;
use bevy::text::FontSource;

/// The active dialogue: the sequence of boxes and which one is showing.
#[derive(Resource, Default)]
pub struct Dialogue {
    pub boxes: Vec<MessageBox>,
    pub index: usize,
    pub active: bool,
}

impl Dialogue {
    /// Show `boxes` from the first one. Called by the event interpreter, which
    /// then pauses until the player dismisses the last box (`active` clears).
    pub fn open(&mut self, boxes: Vec<MessageBox>) {
        self.boxes = boxes;
        self.index = 0;
        self.active = true;
    }
}

#[derive(Component)]
struct DialoguePanel;

#[derive(Component)]
struct DialogueText;

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Dialogue>()
            .add_systems(Startup, spawn_ui)
            .add_systems(Update, (interact, update_ui));
    }
}

/// Spawn the initially hidden dialogue box pinned to the bottom of the screen,
/// styled with the original RM2000 windowskin from `System.png`: a 9-sliced
/// frame behind an opaque blue fill, with the text on top.
fn spawn_ui(mut commands: Commands, font: Res<GameFont>, asset_server: Res<AssetServer>) {
    let system: Handle<Image> = asset_server.load("graphics/System/System.png");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                right: Val::Px(16.0),
                bottom: Val::Px(16.0),
                min_height: Val::Px(96.0),
                padding: UiRect::all(Val::Px(14.0)),
                ..default()
            },
            Visibility::Hidden,
            DialoguePanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                fill_node(),
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
                DialogueText,
            ));
        });
}

/// An absolutely-positioned node filling its parent (the windowskin frame).
fn fill_node() -> Node {
    inset_node(0.0)
}

/// An absolutely-positioned node inset by `px` on every side.
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

/// Advance an open message box on the action key, or — when idle — start the
/// interpreter for an action-key (trigger 0) event on the tile the player faces.
#[allow(clippy::too_many_arguments)]
fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    fade: Res<Fade>,
    map_events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    mut dialogue: ResMut<Dialogue>,
    mut running: ResMut<RunningEvent>,
    players: Query<&Player>,
) {
    if fade.busy() {
        return;
    }
    if !keys.just_pressed(KeyCode::Space) && !keys.just_pressed(KeyCode::Enter) {
        return;
    }
    if dialogue.active {
        dialogue.index += 1;
        if dialogue.index >= dialogue.boxes.len() {
            dialogue.active = false;
            dialogue.boxes.clear();
            dialogue.index = 0;
        }
        return;
    }
    if running.active() {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let (fx, fy) = facing_tile(player);
    for event in &map_events.events {
        if event.x as i32 != fx || event.y as i32 != fy {
            continue;
        }
        if let Some(page) = active_page(event, &switches, &variables, &party, &inventory)
            && page.trigger == 0
        {
            running.start(event.id, page.commands.clone());
            return;
        }
    }
}

fn update_ui(
    dialogue: Res<Dialogue>,
    mut panels: Query<&mut Visibility, With<DialoguePanel>>,
    mut texts: Query<&mut Text, With<DialogueText>>,
) {
    if !dialogue.is_changed() {
        return;
    }
    let showing = dialogue.active && dialogue.index < dialogue.boxes.len();
    if let Ok(mut visibility) = panels.single_mut() {
        *visibility = if showing { Visibility::Visible } else { Visibility::Hidden };
    }
    if showing && let Ok(mut text) = texts.single_mut() {
        **text = dialogue.boxes[dialogue.index].lines.join("\n");
    }
}
