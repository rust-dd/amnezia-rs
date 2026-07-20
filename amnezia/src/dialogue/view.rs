//! The message-box UI: spawning the windowskin box and, each frame, syncing its
//! visibility, face, revealed text, transparency, and the blinking continue
//! arrow to the [`Dialogue`](super::Dialogue) state.
//!
//! Every dimension is RM2000's own, scaled ×3 (our 960×720 window is exactly three
//! times RM2000's 320×240). The box is `Window_Message`'s full-width bottom strip
//! (`MESSAGE_BOX_WIDTH`×`MESSAGE_BOX_HEIGHT` = 320×80 → 960×240); the frame is the
//! 8px `System.png` windowskin border (→ 24px); the face is a 48×48 FaceSet cell
//! (→ 144) drawn at (`LeftMargin`,`TopMargin`) inside the border, i.e. window
//! (16,16) → (48,48); text starts at `LeftMargin+FaceSize+RightFaceMargin` = 72
//! past the border → window 80 → 240 with a face, or at the border (8 → 24)
//! without one; the font is RM2000's 12px on a 16px line pitch (→ 36 / 48).

use super::{Dialogue, MessagePosition, MessageTransparent};
use crate::assets::resolve_png;
use crate::font::GameFont;
use bevy::prelude::*;
use bevy::text::{FontSource, LineHeight};

/// A FaceSet cell is 48×48 in the source sheet; the box renders it at ×3.
const FACE_CELL: f32 = 48.0;
const FACE_BOX: f32 = 144.0;
/// The `System.png` windowskin frame is an 8px border in 320×240 → 24px at ×3.
const BORDER: f32 = 24.0;
/// RM2000 message text: a 12px font on a 16px line pitch → 36 / 48 at ×3.
const FONT_PX: f32 = 36.0;
const LINE_PX: f32 = 48.0;
/// `MESSAGE_BOX_HEIGHT` (80) at ×3: the fixed full-width bottom strip.
const BOX_H: f32 = 240.0;
/// Face origin (window (16,16) at ×3) and the text's left with / without a face.
const FACE_X: f32 = 48.0;
const FACE_Y: f32 = 48.0;
const TEXT_X: f32 = 24.0;
const TEXT_X_FACE: f32 = 240.0;
const TEXT_Y: f32 = 24.0;

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

/// Spawn the initially hidden dialogue box: RM2000's full-width bottom strip
/// (`Window_Message`) styled with the `System.png` windowskin — a 9-sliced frame
/// over the stretched background fill, the face and text on top, and the continue
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
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(BOX_H),
                ..default()
            },
            Visibility::Hidden,
            DialoguePanel,
        ))
        .with_children(|panel| {
            // Background fill first, then the frame on top: Bevy draws later
            // siblings above earlier ones, and the 9-sliced frame's centre is
            // transparent, so the fill shows through the middle while the 24px
            // border stays visible. (Spawning the fill last painted over it.)
            panel.spawn((
                fill_node(),
                ImageNode {
                    image: system.clone(),
                    rect: Some(Rect::new(0.0, 0.0, 32.0, 32.0)),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
                DialogueFrame,
            ));
            panel.spawn((
                fill_node(),
                ImageNode {
                    image: system.clone(),
                    rect: Some(Rect::new(32.0, 0.0, 64.0, 32.0)),
                    image_mode: NodeImageMode::Sliced(TextureSlicer {
                        border: BorderRect::all(8.0),
                        center_scale_mode: SliceScaleMode::Stretch,
                        sides_scale_mode: SliceScaleMode::Stretch,
                        // 8px source border × the ×3 scale = RM2000's 24px frame.
                        max_corner_scale: 3.0,
                    }),
                    ..default()
                },
                DialogueFrame,
            ));
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
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(font.0.clone()),
                    font_size: FontSize::Px(FONT_PX),
                    ..default()
                },
                TextColor(Color::WHITE),
                LineHeight::Px(LINE_PX),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(TEXT_X),
                    top: Val::Px(TEXT_Y),
                    right: Val::Px(BORDER),
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

/// An absolutely-positioned node filling its parent (the windowskin frame).
fn fill_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}

/// Sync the box's per-page presentation when the shown box, its face, or the
/// transparent-box flag changes: panel visibility, the face graphic, the text's
/// face-offset left edge, and whether the windowskin frame is drawn. Guarded by a
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
    if let Ok(mut node) = texts.single_mut() {
        node.left = Val::Px(if face.is_some() { TEXT_X_FACE } else { TEXT_X });
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
/// [`MessagePosition`] changes, mirroring `Window_Message`'s three fixed slots for
/// its 240px-tall box: top edge, screen centre ((240−80)/2 = 80 → 240 at ×3), or
/// flush against the bottom (the spawn default).
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
            node.top = Val::Px(0.0);
            node.bottom = Val::Auto;
        }
        MessagePosition::Middle => {
            node.top = Val::Px(240.0);
            node.bottom = Val::Auto;
        }
        MessagePosition::Bottom => {
            node.top = Val::Auto;
            node.bottom = Val::Px(0.0);
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
