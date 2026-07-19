//! The message-box UI: spawning the windowskin box and, each frame, syncing its
//! visibility, face, revealed text, transparency, and the blinking continue
//! arrow to the [`Dialogue`](super::Dialogue) state.

use super::{Dialogue, MessagePosition, MessageTransparent};
use crate::assets::resolve_png;
use crate::font::GameFont;
use bevy::prelude::*;
use bevy::text::FontSource;

/// One RM2000 FaceSet face is 48×48 pixels, laid out in a 4×4 grid.
const FACE_SIZE: f32 = 48.0;

/// Frames the continue arrow stays visible, then hidden, per blink half-cycle
/// (EasyRPG's `arrow_animation_frames`).
const ARROW_BLINK_FRAMES: u32 = 20;

#[derive(Component)]
pub(super) struct DialoguePanel;

#[derive(Component)]
pub(super) struct DialogueText;

#[derive(Component)]
pub(super) struct DialogueFace;

/// The windowskin fill and frame layers, hidden together when a message uses the
/// transparent-box option so only the text and face remain.
#[derive(Component)]
pub(super) struct DialogueFrame;

/// The blinking "▼" continue indicator, shown at the box's bottom edge once a
/// page is fully revealed and waiting for the confirm key.
#[derive(Component)]
pub(super) struct DialogueArrow;

/// Spawn the initially hidden dialogue box pinned to the bottom of the screen,
/// styled with the original RM2000 windowskin from `System.png`: a 9-sliced
/// frame behind an opaque blue fill, the face and text on top, and the continue
/// arrow at the bottom edge.
pub(super) fn spawn_ui(
    mut commands: Commands,
    font: Res<GameFont>,
    asset_server: Res<AssetServer>,
) {
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
                DialogueFrame,
            ));
            panel.spawn((
                inset_node(4.0),
                ImageNode {
                    image: system.clone(),
                    rect: Some(Rect::new(0.0, 0.0, 32.0, 32.0)),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
                DialogueFrame,
            ));
            panel.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(4.0),
                    top: Val::Px(4.0),
                    width: Val::Px(FACE_SIZE),
                    height: Val::Px(FACE_SIZE),
                    ..default()
                },
                ImageNode::default(),
                Visibility::Hidden,
                DialogueFace,
            ));
            panel.spawn((
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(font.0.clone()),
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node { ..default() },
                DialogueText,
            ));
            panel.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(2.0),
                    left: Val::Percent(50.0),
                    margin: UiRect::left(Val::Px(-11.0)),
                    width: Val::Px(22.0),
                    height: Val::Px(11.0),
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

/// Sync the box's per-page presentation when the shown box, its face, or the
/// transparent-box flag changes: panel visibility, the face graphic, the text's
/// face-offset margin, and whether the windowskin frame is drawn. Guarded by a
/// remembered snapshot so it does no work while a page reveals letter by letter.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn render_box(
    dialogue: Res<Dialogue>,
    transparent: Res<MessageTransparent>,
    asset_server: Res<AssetServer>,
    mut last: Local<Option<(bool, usize, bool)>>,
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
    mut texts: Query<&mut Node, With<DialogueText>>,
) {
    let showing = dialogue.active && dialogue.index < dialogue.boxes.len();
    let snapshot = (showing, dialogue.index, transparent.0);
    if *last == Some(snapshot) {
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

    let current = showing
        .then(|| dialogue.boxes.get(dialogue.index))
        .flatten();
    let face = current.and_then(|b| b.face.as_ref().map(|name| (name.clone(), b.face_index)));
    if let Ok((mut image, mut visibility)) = faces.single_mut() {
        match &face {
            Some((name, index)) => {
                image.image = asset_server.load(resolve_png("FaceSet", name));
                let (col, row) = ((index % 4) as f32, (index / 4) as f32);
                image.rect = Some(Rect::new(
                    col * FACE_SIZE,
                    row * FACE_SIZE,
                    col * FACE_SIZE + FACE_SIZE,
                    row * FACE_SIZE + FACE_SIZE,
                ));
                *visibility = Visibility::Visible;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    if let Ok(mut node) = texts.single_mut() {
        node.margin.left = if face.is_some() {
            Val::Px(FACE_SIZE + 8.0)
        } else {
            Val::Px(0.0)
        };
    }
}

/// Each frame, show the glyphs the reveal has uncovered and blink the continue
/// arrow while a fully-revealed page (or a `\!` pause) waits for the confirm key.
#[allow(clippy::type_complexity)]
pub(super) fn render_reveal(
    dialogue: Res<Dialogue>,
    mut frame: Local<u32>,
    mut texts: Query<&mut Text, With<DialogueText>>,
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
        let shown = reveal.map(|r| r.text()).unwrap_or("");
        if text.as_str() != shown {
            **text = shown.to_string();
        }
    }

    *frame = frame.wrapping_add(1);
    let waiting =
        reveal.is_some_and(|r| r.waiting_for_key() || (r.is_complete() && !r.kill_page()));
    let blink_on = (*frame / ARROW_BLINK_FRAMES).is_multiple_of(2);
    if let Ok(mut visibility) = arrows.single_mut() {
        *visibility = visible_if(waiting && blink_on);
    }
}

/// Anchor the dialogue box to the top, middle, or bottom of the screen when
/// [`MessagePosition`] changes. Bottom (the spawn default) pins it to the
/// bottom; Top pins it to the top; Middle centres it, nudged up by half the
/// box's min height so the box straddles the centre line.
pub(super) fn update_position(
    position: Res<MessagePosition>,
    mut panels: Query<&mut Node, With<DialoguePanel>>,
) {
    if !position.is_changed() {
        return;
    }
    let Ok(mut node) = panels.single_mut() else {
        return;
    };
    match *position {
        MessagePosition::Top => {
            node.top = Val::Px(16.0);
            node.bottom = Val::Auto;
            node.margin.top = Val::Px(0.0);
        }
        MessagePosition::Middle => {
            node.top = Val::Percent(50.0);
            node.bottom = Val::Auto;
            node.margin.top = Val::Px(-48.0);
        }
        MessagePosition::Bottom => {
            node.top = Val::Auto;
            node.bottom = Val::Px(16.0);
            node.margin.top = Val::Px(0.0);
        }
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
