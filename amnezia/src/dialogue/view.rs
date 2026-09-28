//! Original bitmap message contents, portrait and windowskin layers in native
//! coordinates scaled threefold for Bevy UI.

use super::{Dialogue, MessageTransparent};
use crate::assets::resolve_png;
use crate::font::bitmap::{DEFAULT, PixelText, Run};
use bevy::prelude::*;

#[cfg(test)]
mod arrow_tests;
pub(super) mod motion;
pub(super) mod prompts;
pub(crate) mod smoke;
#[cfg(test)]
mod tests;

/// A FaceSet cell is 48×48 in the source sheet; the box renders it at ×3.
const FACE_CELL: f32 = 48.0;
const FACE_BOX: f32 = 144.0;
/// The `System.png` windowskin frame is an 8px border in 320×240 → 24px at ×3.
const BORDER: f32 = 24.0;
/// `MESSAGE_BOX_HEIGHT` (80) at ×3: the fixed full-width bottom strip.
const BOX_H: f32 = 240.0;
/// Face origin inside the window contents, including its eight-pixel border.
const FACE_X: f32 = 48.0;
const FACE_Y: f32 = 48.0;
const CONTENTS_WIDTH: u32 = 304;
const CONTENTS_HEIGHT: u32 = 64;

#[derive(Component)]
pub(crate) struct DialoguePanel;

#[derive(Component)]
pub(super) struct DialogueText;

#[derive(Component)]
pub(super) struct DialogueFace;

/// The windowskin fill and frame layers, hidden together when a message uses the
/// transparent-box option so only the text and face remain.
#[derive(Component)]
pub(super) struct DialogueFrame;

/// The pause indicator at the window's bottom edge, including final text delays.
#[derive(Component)]
pub(super) struct DialogueArrow;

/// Spawn the hidden window, clipped bitmap contents, face and continue arrow.
pub(super) fn spawn_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let system = asset_server.load::<Image>("graphics/System/System.png");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(BOX_H),
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(100),
            DialoguePanel,
        ))
        .with_children(|panel| {
            panel
                .spawn((fill_node(), DialogueFrame))
                .with_children(|frame| {
                    crate::windowskin::fixed_frame(frame, &system, UVec2::new(320, 80))
                });
            prompts::spawn(panel, &system);
            motion::spawn(panel);
            panel.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(FACE_X),
                    top: Val::Px(FACE_Y),
                    width: Val::Px(FACE_BOX),
                    height: Val::Px(FACE_BOX),
                    ..default()
                },
                ImageNode::default(),
                Visibility::Hidden,
                DialogueFace,
            ));
            panel.spawn((
                PixelText {
                    size: UVec2::new(CONTENTS_WIDTH, CONTENTS_HEIGHT),
                    runs: Vec::new(),
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(BORDER),
                    top: Val::Px(BORDER),
                    width: Val::Px(CONTENTS_WIDTH as f32 * 3.0),
                    height: Val::Px(CONTENTS_HEIGHT as f32 * 3.0),
                    ..default()
                },
                DialogueText,
            ));
            panel.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(0.0),
                    left: Val::Percent(50.0),
                    margin: UiRect::left(Val::Px(-24.0)),
                    width: Val::Px(48.0),
                    height: Val::Px(24.0),
                    ..default()
                },
                ImageNode {
                    image: system.clone(),
                    rect: Some(Rect::new(40.0, 16.0, 56.0, 24.0)),
                    ..default()
                },
                Visibility::Hidden,
                DialogueArrow,
            ));
        });
}

pub(super) fn target_camera(
    mut commands: Commands,
    battle: Res<crate::battle::BattleActive>,
    cameras: Query<Entity, With<crate::battle::HudCamera>>,
    panels: Query<Entity, With<DialoguePanel>>,
) {
    if !battle.is_changed() {
        return;
    }
    for panel in &panels {
        if battle.0 {
            if let Ok(camera) = cameras.single() {
                commands.entity(panel).insert(UiTargetCamera(camera));
            }
        } else {
            commands.entity(panel).remove::<UiTargetCamera>();
        }
    }
}

/// An absolutely-positioned node filling its parent (the windowskin frame).
fn fill_node() -> Node {
    inset_node(0.0)
}

/// An absolutely-positioned node inset by `px` on every side of its parent (the
/// background fill, tucked inside the frame's border).
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

/// Update visibility and portrait only when the page presentation changes.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn render_box(
    dialogue: Res<Dialogue>,
    prompts: prompts::Presentation,
    transparent: Res<MessageTransparent>,
    asset_server: Res<AssetServer>,
    mut last: Local<Option<(bool, u64, usize, bool, bool, Option<(String, u32)>)>>,
    mut panels: Query<
        &mut Visibility,
        (
            With<DialoguePanel>,
            Without<DialogueFace>,
            Without<DialogueFrame>,
        ),
    >,
    mut frames: Query<
        &mut Visibility,
        (
            With<DialogueFrame>,
            Without<DialoguePanel>,
            Without<DialogueFace>,
        ),
    >,
    mut faces: Query<
        (&mut ImageNode, &mut Visibility),
        (
            With<DialogueFace>,
            Without<DialoguePanel>,
            Without<DialogueFrame>,
        ),
    >,
) {
    let showing = dialogue.lifecycle.message.visible()
        || (dialogue.active && dialogue.index < dialogue.boxes.len())
        || prompts.active();
    let face = prompts.face(&dialogue);
    let snapshot = (
        showing,
        dialogue.generation,
        dialogue.index,
        transparent.0,
        dialogue.lifecycle.message.ready(),
        face.map(|(name, index)| (name.to_string(), index)),
    );
    if last.as_ref() == Some(&snapshot) {
        return;
    }
    *last = Some(snapshot);

    if let Ok(mut visibility) = panels.single_mut() {
        *visibility = visible_if(showing);
    }
    let frame_shown = showing && !transparent.0;
    for mut visibility in &mut frames {
        *visibility = visible_if(frame_shown);
    }

    if let Ok((mut image, mut visibility)) = faces.single_mut() {
        match face.filter(|_| !dialogue.active || dialogue.lifecycle.message.ready()) {
            Some((name, index)) => {
                image.image = asset_server.load(resolve_png("FaceSet", name));
                let (col, row) = ((index % 4) as f32, (index / 4) as f32);
                image.rect = Some(Rect::new(
                    col * FACE_CELL,
                    row * FACE_CELL,
                    col * FACE_CELL + FACE_CELL,
                    row * FACE_CELL + FACE_CELL,
                ));
                *visibility = Visibility::Visible;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// Render the bitmap contents and the logical pause-arrow state.
#[allow(clippy::type_complexity)]
pub(super) fn render_reveal(
    dialogue: Res<Dialogue>,
    prompts: prompts::Presentation,
    mut texts: Query<&mut PixelText, With<DialogueText>>,
    mut arrows: Query<
        &mut Visibility,
        (
            With<DialogueArrow>,
            Without<DialoguePanel>,
            Without<DialogueFace>,
            Without<DialogueFrame>,
        ),
    >,
) {
    let reveal = dialogue
        .active
        .then_some(dialogue.reveal.as_ref())
        .flatten();
    if let Ok(mut text) = texts.single_mut() {
        let face = prompts.face(&dialogue).is_some();
        let next = PixelText {
            size: UVec2::new(CONTENTS_WIDTH, CONTENTS_HEIGHT),
            runs: if dialogue.embedded_prompt().is_some() {
                prompts.embedded_runs(&dialogue, face)
            } else if prompts.active() {
                prompts.runs(face)
            } else {
                reveal.map_or_else(Vec::new, |reveal| {
                    vec![Run::new(
                        reveal.text(),
                        if face { 72 } else { 0 },
                        2,
                        DEFAULT,
                    )]
                })
            },
        };
        if *text != next {
            *text = next;
        }
    }

    if let Ok(mut visibility) = arrows.single_mut() {
        *visibility =
            visible_if(!prompts.active() && reveal.is_some_and(|reveal| reveal.arrow_visible()));
    }
}

/// Map a shown/hidden flag to a [`Visibility`].
fn visible_if(shown: bool) -> Visibility {
    if shown {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}
