//! Original bitmap message contents, portrait and windowskin layers in native
//! coordinates scaled threefold for Bevy UI.

use super::{Dialogue, MessagePosition, MessageTransparent};
use crate::assets::resolve_png;
use crate::font::bitmap::{DEFAULT, PixelText, Run};
use bevy::prelude::*;

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
                .with_children(|frame| crate::windowskin::frame(frame, &system));
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
    transparent: Res<MessageTransparent>,
    asset_server: Res<AssetServer>,
    mut last: Local<Option<(bool, u64, usize, bool)>>,
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
    let showing = dialogue.active && dialogue.index < dialogue.boxes.len();
    let snapshot = (showing, dialogue.generation, dialogue.index, transparent.0);
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
}

/// Each frame, show the glyphs the reveal has uncovered and blink the continue
/// arrow while a fully-revealed page (or a `\!` pause) waits for the confirm key.
#[allow(clippy::type_complexity)]
pub(super) fn render_reveal(
    dialogue: Res<Dialogue>,
    mut frame: Local<u32>,
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
        let face = dialogue
            .boxes
            .get(dialogue.index)
            .is_some_and(|page| page.face.is_some());
        let next = PixelText {
            size: UVec2::new(CONTENTS_WIDTH, CONTENTS_HEIGHT),
            runs: reveal.map_or_else(Vec::new, |reveal| {
                vec![Run::new(
                    reveal.text(),
                    if face { 72 } else { 0 },
                    2,
                    DEFAULT,
                )]
            }),
        };
        if *text != next {
            *text = next;
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

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn update_position(
    position: Res<MessagePosition>,
    options: Res<super::MessageOptions>,
    dialogue: Res<Dialogue>,
    map: Option<Res<crate::world::MapData>>,
    screen: crate::world::MapScreen,
    battle: Option<Res<crate::battle::BattleActive>>,
    mut previous: Local<Option<(u64, usize, MessagePosition, super::MessageOptions, bool)>>,
    mut panels: Query<&mut Node, With<DialoguePanel>>,
) {
    if !dialogue.active {
        *previous = None;
        return;
    }
    let battle = battle.is_some_and(|b| b.0);
    let snapshot = (
        dialogue.generation,
        dialogue.index,
        *position,
        *options,
        battle,
    );
    if *previous == Some(snapshot) {
        return;
    }
    *previous = Some(snapshot);
    let hero_y = map
        .as_deref()
        .and_then(|map| screen.hero_y(map))
        .unwrap_or(128);
    let position = options.position(*position, hero_y, battle);
    let Ok(mut node) = panels.single_mut() else {
        return;
    };
    match position {
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
