//! Numeric entry: the windowskin box the interpreter opens for an `InputNumber`
//! command (RM2000 opcode `10150`). It shows a row of digit slots the player
//! edits with the arrow keys and confirms with the action key; the interpreter
//! reads the assembled number back and stores it in the target variable.

use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::font::GameFont;
use bevy::prelude::*;
use bevy::text::FontSource;

/// The active numeric entry box: how many digit slots it has, which variable the
/// result is destined for, the per-slot digits and the cursor slot, the running
/// assembled `value`, whether it's showing, and — once the player confirms — the
/// entered number the interpreter consumes.
#[derive(Resource, Default)]
pub struct InputNumber {
    /// Slot count the box opened with; kept for parity with the RM2000 command
    /// though the UI derives its width from `slots`.
    #[allow(dead_code)]
    pub digits: u32,
    pub var_id: u32,
    pub value: i64,
    pub active: bool,
    pub result: Option<i64>,
    slots: Vec<u8>,
    cursor: usize,
}

impl InputNumber {
    /// Show a numeric entry box of `digits` slots writing into variable `var_id`.
    /// The interpreter pauses until the player confirms (`active` clears and
    /// `result` is set to the assembled number).
    pub fn open(&mut self, digits: u32, var_id: u32) {
        self.digits = digits;
        self.var_id = var_id;
        self.slots = vec![0u8; digits as usize];
        self.cursor = 0;
        self.value = 0;
        self.active = true;
        self.result = None;
    }

    /// Whether the box is showing (the interpreter and movement pause).
    pub fn active(&self) -> bool {
        self.active
    }

    /// Nudge the cursor slot's digit by `delta`, wrapping within `0..=9`, then
    /// recompute the assembled value.
    fn adjust(&mut self, delta: i8) {
        let cursor = self.cursor;
        let Some(slot) = self.slots.get_mut(cursor) else {
            return;
        };
        *slot = (*slot as i8 + delta).rem_euclid(10) as u8;
        self.recompute();
    }

    /// Reassemble `value` from the digit slots (most significant slot first).
    fn recompute(&mut self) {
        self.value = self.slots.iter().fold(0i64, |acc, &d| acc * 10 + d as i64);
    }
}

#[derive(Component)]
struct InputNumberPanel;

#[derive(Component)]
struct InputNumberText;

pub struct InputNumberPlugin;

impl Plugin for InputNumberPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputNumber>()
            .add_systems(Startup, spawn_ui)
            .add_systems(Update, (input_number_input, update_ui));
    }
}

/// Spawn the initially hidden entry box, centered on screen, styled with the same
/// RM2000 windowskin (`System.png`) as the dialogue and choice boxes.
fn spawn_ui(mut commands: Commands, font: Res<GameFont>, asset_server: Res<AssetServer>) {
    let system: Handle<Image> = asset_server.load("graphics/System/System.png");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(50),
            InputNumberPanel,
        ))
        .with_children(|screen| {
            screen
                .spawn(Node {
                    min_width: Val::Px(120.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
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
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        InputNumberText,
                    ));
                });
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

/// Edit the current digit with ↑/↓ (wrapping `0..=9`), move between slots with
/// ←/→, and confirm with the action key, storing the assembled number in
/// `result` and closing the box. Cursor moves play the RM2000 cursor SE and
/// confirming plays the decision SE, like RPG_RT's number-input window.
fn input_number_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<InputNumber>,
    mut audio: MessageWriter<AudioRequest>,
    sounds: Option<Res<SystemSounds>>,
) {
    if !input.active {
        return;
    }
    let sounds = sounds.as_deref();
    let confirm = keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter);
    let count = input.slots.len();
    if count == 0 {
        if confirm {
            if let Some(s) = sounds {
                play_system_se(&mut audio, &s.decision);
            }
            input.result = Some(0);
            input.active = false;
        }
        return;
    }
    let mut moved = false;
    if keys.just_pressed(KeyCode::ArrowUp) {
        input.adjust(1);
        moved = true;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        input.adjust(-1);
        moved = true;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        input.cursor = (input.cursor + 1) % count;
        moved = true;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        input.cursor = (input.cursor + count - 1) % count;
        moved = true;
    }
    if moved && let Some(s) = sounds {
        play_system_se(&mut audio, &s.cursor);
    }
    if confirm {
        if let Some(s) = sounds {
            play_system_se(&mut audio, &s.decision);
        }
        input.result = Some(input.value);
        input.active = false;
    }
}

fn update_ui(
    input: Res<InputNumber>,
    mut panels: Query<&mut Visibility, With<InputNumberPanel>>,
    mut texts: Query<&mut Text, With<InputNumberText>>,
) {
    if !input.is_changed() {
        return;
    }
    if let Ok(mut visibility) = panels.single_mut() {
        *visibility = if input.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if input.active
        && let Ok(mut text) = texts.single_mut()
    {
        let mut rendered = String::new();
        for (i, &d) in input.slots.iter().enumerate() {
            let digit = (b'0' + d) as char;
            if i == input.cursor {
                rendered.push('[');
                rendered.push(digit);
                rendered.push(']');
            } else {
                rendered.push(' ');
                rendered.push(digit);
                rendered.push(' ');
            }
        }
        **text = rendered;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tap(app: &mut App, key: KeyCode) {
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear();
            keys.press(key);
        }
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
    }

    fn headless() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<InputNumber>();
        app.add_message::<AudioRequest>();
        app.add_systems(Update, input_number_input);
        app
    }

    #[test]
    fn enters_two_digit_number() {
        let mut app = headless();
        app.world_mut().resource_mut::<InputNumber>().open(2, 20);

        for _ in 0..4 {
            tap(&mut app, KeyCode::ArrowUp);
        }
        tap(&mut app, KeyCode::ArrowRight);
        for _ in 0..2 {
            tap(&mut app, KeyCode::ArrowUp);
        }
        tap(&mut app, KeyCode::Enter);

        let input = app.world().resource::<InputNumber>();
        assert_eq!(input.result, Some(42));
        assert!(!input.active());
        assert_eq!(input.var_id, 20);
    }

    #[test]
    fn arrow_down_wraps_to_nine() {
        let mut app = headless();
        app.world_mut().resource_mut::<InputNumber>().open(1, 7);

        tap(&mut app, KeyCode::ArrowDown);
        tap(&mut app, KeyCode::Space);

        let input = app.world().resource::<InputNumber>();
        assert_eq!(input.result, Some(9));
        assert!(!input.active());
    }
}
