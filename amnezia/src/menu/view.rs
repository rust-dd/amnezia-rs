//! The menu panel's spawned scaffold: two side-by-side windowskin windows in the
//! RM2000 split — a left command/content window and a right party-status window —
//! each framed with the same `System.png` slices as the dialogue box, each with a
//! single text child the screens compose into. State and input live in the parent
//! module; this one only builds and marks the nodes. The parent hides the right
//! window on sub-screens (where the party column is empty).

use crate::font::GameFont;
use bevy::prelude::*;
use bevy::text::FontSource;

#[derive(Component)]
pub(super) struct MenuPanel;

/// The right (party status) window container, hidden by the parent on sub-screens.
#[derive(Component)]
pub(super) struct MenuAuxPanel;

/// The left/main window's text.
#[derive(Component)]
pub(super) struct MenuText;

/// The right (party status) window's text.
#[derive(Component)]
pub(super) struct MenuAux;

/// Spawn the initially hidden, near-fullscreen menu overlay: a horizontal pair of
/// windowskin windows, the left one for the active screen, the right one for the
/// party roster.
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
                left: Val::Px(24.0),
                right: Val::Px(24.0),
                top: Val::Px(24.0),
                bottom: Val::Px(24.0),
                column_gap: Val::Px(8.0),
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(100),
            MenuPanel,
        ))
        .with_children(|panel| {
            panel
                .spawn((window_node(), Visibility::Inherited))
                .with_children(|window| {
                    frame(window, &system);
                    window.spawn((text_node(&font), MenuText));
                });
            panel
                .spawn((window_node(), Visibility::Inherited, MenuAuxPanel))
                .with_children(|window| {
                    frame(window, &system);
                    window.spawn((text_node(&font), MenuAux));
                });
        });
}

/// One windowskin window: an equal-width flex column that positions its absolute
/// frame images and holds its in-flow text.
fn window_node() -> Node {
    Node {
        position_type: PositionType::Relative,
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_width: Val::Px(0.0),
        padding: UiRect::all(Val::Px(16.0)),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// Spawn a window's two windowskin layers: the sliced border and the stretched
/// centre fill, both absolutely filling the window behind its text.
fn frame(window: &mut ChildSpawnerCommands, system: &Handle<Image>) {
    window.spawn((
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
    window.spawn((
        inset_node(4.0),
        ImageNode {
            image: system.clone(),
            rect: Some(Rect::new(0.0, 0.0, 32.0, 32.0)),
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
    ));
}

/// The text bundle a window composes its screen into.
fn text_node(font: &GameFont) -> impl Bundle {
    (
        Text::new(String::new()),
        TextFont {
            font: FontSource::Handle(font.0.clone()),
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::WHITE),
    )
}

/// An absolutely-positioned node inset by `px` on every side of its parent.
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
