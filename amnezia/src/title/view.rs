use super::*;
use crate::font::GameFont;
use bevy::text::FontSource;

#[derive(Component)]
pub(super) struct TitleRoot;

#[derive(Component)]
pub(super) struct TitleRow(usize);

const ENABLED: Color = Color::WHITE;
const DISABLED: Color = Color::srgb(0.5, 0.5, 0.5);

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
    let has_save = crate::save::save_slot_exists();
    let cursor = default_cursor(has_save);
    commands
        .spawn((
            fill_node(),
            BackgroundColor(Color::BLACK),
            GlobalZIndex(2000),
            Visibility::Hidden,
            TitleRoot,
        ))
        .with_children(|root| {
            root.spawn((
                fill_node(),
                ImageNode {
                    image: assets.load(crate::assets::resolve_png("Title", "Title")),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(56.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|menu| {
                for (i, label) in ROWS.iter().enumerate() {
                    menu.spawn((
                        Text::new(row_text(label, i == cursor)),
                        TextFont {
                            font: FontSource::Handle(font.0.clone()),
                            font_size: FontSize::Px(crate::font::UI_FONT_PX),
                            ..default()
                        },
                        TextColor(if i != CONTINUE || has_save {
                            ENABLED
                        } else {
                            DISABLED
                        }),
                        bevy::text::LineHeight::Px(crate::font::UI_LINE_PX),
                        TitleRow(i),
                    ));
                }
            });
        });
}

pub(super) fn update(
    title: Res<TitleActive>,
    state: Res<TitleState>,
    mut roots: Query<&mut Visibility, With<TitleRoot>>,
    mut rows: Query<(&TitleRow, &mut Text, &mut TextColor)>,
) {
    if !title.is_changed() && !state.is_changed() {
        return;
    }
    if let Ok(mut visibility) = roots.single_mut() {
        *visibility = if title.0 && state.stage.visible() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let has_save = crate::save::save_slot_exists();
    for (row, mut text, mut color) in &mut rows {
        **text = row_text(ROWS[row.0], row.0 == state.cursor);
        *color = TextColor(if row.0 != CONTINUE || has_save {
            ENABLED
        } else {
            DISABLED
        });
    }
}
