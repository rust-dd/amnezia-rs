//! Dialogue choices: the windowskin menu the interpreter opens for a
//! `ShowChoice` command. It renders at the message box's position with the
//! options listed under a cursor the player moves with the arrow keys and
//! confirms with the action key; the cancel key selects the choice's configured
//! cancel option (or is refused when the choice disallows cancelling). The
//! interpreter reads the chosen index back and runs the matching branch.

use crate::font::GameFont;
use bevy::prelude::*;
use bevy::text::FontSource;

mod input;
pub(crate) mod smoke;
use input::update as choice_input;

/// The active choice menu: the option labels, the cursor row, which event
/// `indent` this choice belongs to, its RM2000 cancel type, whether it's showing,
/// and — once the player confirms or cancels — the chosen option index the
/// interpreter consumes.
#[derive(Resource, Default)]
pub struct Choice {
    pub options: Vec<String>,
    pub cursor: usize,
    pub indent: u32,
    /// RM2000 `ShowChoices` cancel type (`parameters[0]`): `0` disallows cancel,
    /// otherwise the cancel key picks option `cancel_type - 1` (the special cancel
    /// branch when it exceeds the listed options).
    pub cancel_type: i32,
    pub active: bool,
    pub result: Option<i32>,
}

impl Choice {
    /// Show `options` for the choice at event `indent` with the given RM2000
    /// `cancel_type`. The interpreter pauses until the player confirms or cancels
    /// (`active` clears and `result` is set).
    pub fn open(&mut self, options: Vec<String>, indent: u32, cancel_type: i32) {
        self.options = options;
        self.cursor = 0;
        self.indent = indent;
        self.cancel_type = cancel_type;
        self.active = true;
        self.result = None;
    }

    /// Whether the menu is showing (the interpreter and movement pause).
    pub fn active(&self) -> bool {
        self.active
    }
}

#[derive(Component)]
struct ChoicePanel;

#[derive(Component)]
struct ChoiceText;

pub struct ChoicePlugin;

impl Plugin for ChoicePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Choice>()
            .add_systems(Startup, spawn_ui)
            .add_systems(
                Update,
                (choice_input, update_ui)
                    .chain()
                    .after(crate::menu::MenuInput),
            );
    }
}

/// Spawn the initially hidden choice box at the bottom message-box position,
/// styled with the same RM2000 windowskin (`System.png`) as the dialogue box, so
/// the options read as continuing below the message they follow.
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
            GlobalZIndex(50),
            ChoicePanel,
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
                    font_size: FontSize::Px(crate::font::UI_FONT_PX),
                    ..default()
                },
                TextColor(Color::WHITE),
                bevy::text::LineHeight::Px(crate::font::UI_LINE_PX),
                ChoiceText,
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

fn update_ui(
    choice: Res<Choice>,
    mut panels: Query<&mut Visibility, With<ChoicePanel>>,
    mut texts: Query<&mut Text, With<ChoiceText>>,
) {
    if !choice.is_changed() {
        return;
    }
    if let Ok(mut visibility) = panels.single_mut() {
        *visibility = if choice.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if choice.active
        && let Ok(mut text) = texts.single_mut()
    {
        let mut rendered = String::new();
        for (i, option) in choice.options.iter().enumerate() {
            rendered.push_str(if i == choice.cursor { "▶ " } else { "  " });
            rendered.push_str(option);
            if i + 1 < choice.options.len() {
                rendered.push('\n');
            }
        }
        **text = rendered;
    }
}

#[cfg(test)]
mod tests;
