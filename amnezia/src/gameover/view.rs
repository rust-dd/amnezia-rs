use super::*;
use crate::assets::resolve_png;
use crate::font::GameFont;
use bevy::text::FontSource;

#[derive(Component)]
pub(super) struct GameOverRoot;

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

pub(super) fn spawn(mut commands: Commands, font: Res<GameFont>, assets: Res<AssetServer>) {
    commands
        .spawn((
            fill_node(),
            BackgroundColor(Color::BLACK),
            Visibility::Hidden,
            GlobalZIndex(3000),
            GameOverRoot,
        ))
        .with_children(|root| {
            root.spawn(Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..fill_node()
            })
            .with_children(|center| {
                center.spawn((
                    Text::new("Game Over"),
                    TextFont {
                        font: FontSource::Handle(font.0.clone()),
                        font_size: FontSize::Px(crate::font::UI_FONT_PX),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
            root.spawn((
                fill_node(),
                ImageNode {
                    image: assets.load(resolve_png("GameOver", "GameOver")),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
        });
}

pub(super) fn update(
    flow: Res<GameOverFlow>,
    mut roots: Query<&mut Visibility, With<GameOverRoot>>,
) {
    if !flow.is_changed() {
        return;
    }
    if let Ok(mut visibility) = roots.single_mut() {
        *visibility = if flow.visible() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
