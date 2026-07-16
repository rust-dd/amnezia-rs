//! Dialogue: the message-box UI and the interaction that opens it when the
//! player presses the action key facing an event with dialogue.

use crate::events::{message_boxes, MessageBox};
use crate::font::GameFont;
use crate::player::{facing_tile, Player};
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

/// Spawn the initially hidden dialogue box pinned to the bottom of the screen.
fn spawn_ui(mut commands: Commands, font: Res<GameFont>) {
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
            BackgroundColor(Color::srgba(0.03, 0.03, 0.12, 0.9)),
            Visibility::Hidden,
            DialoguePanel,
        ))
        .with_children(|panel| {
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

fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    map_events: Res<MapEvents>,
    mut dialogue: ResMut<Dialogue>,
    players: Query<&Player>,
) {
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
