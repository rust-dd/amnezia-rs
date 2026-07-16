//! Dialogue: the message-box UI and the interaction that opens it when the
//! player presses the action key facing an event with dialogue.

use crate::events::{message_boxes, teleport_target, MessageBox};
use crate::font::GameFont;
use crate::player::{facing_tile, Player};
use crate::teleport::{Fade, PendingTeleport};
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

fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    fade: Res<Fade>,
    map_events: Res<MapEvents>,
    mut dialogue: ResMut<Dialogue>,
    mut pending: ResMut<PendingTeleport>,
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
    let Ok(player) = players.single() else {
        return;
    };
    let (fx, fy) = facing_tile(player);
    for event in &map_events.events {
        if event.x as i32 != fx || event.y as i32 != fy {
            continue;
        }
        let Some(page) = event.pages.last() else {
            continue;
        };
        if page.trigger != 0 {
            continue;
        }
        let boxes = message_boxes(&page.commands);
        if !boxes.is_empty() {
            dialogue.boxes = boxes;
            dialogue.index = 0;
            dialogue.active = true;
            return;
        }
        if let Some(target) = teleport_target(&page.commands) {
            pending.0 = Some(target);
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
