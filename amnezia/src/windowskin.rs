use bevy::prelude::*;

fn skin_piece(system: &Handle<Image>, rect: Rect) -> ImageNode {
    ImageNode {
        image: system.clone(),
        rect: Some(rect),
        image_mode: NodeImageMode::Stretch,
        ..default()
    }
}

pub(crate) fn frame(parent: &mut ChildSpawnerCommands, system: &Handle<Image>) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        skin_piece(system, Rect::new(0.0, 0.0, 32.0, 32.0)),
    ));
    // The frame's centre contains scroll arrows, not window background.
    border(parent, system, 32.0);
}

pub(crate) fn cursor(parent: &mut ChildSpawnerCommands, system: &Handle<Image>) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(24.0),
            right: Val::Px(24.0),
            top: Val::Px(24.0),
            bottom: Val::Px(24.0),
            ..default()
        },
        skin_piece(system, Rect::new(72.0, 8.0, 88.0, 24.0)),
    ));
    border(parent, system, 64.0);
}

fn border(parent: &mut ChildSpawnerCommands, system: &Handle<Image>, origin: f32) {
    for x in 0..2 {
        for y in 0..2 {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: if x == 0 { Val::Px(0.0) } else { Val::Auto },
                    right: if x == 1 { Val::Px(0.0) } else { Val::Auto },
                    top: if y == 0 { Val::Px(0.0) } else { Val::Auto },
                    bottom: if y == 1 { Val::Px(0.0) } else { Val::Auto },
                    width: Val::Px(24.0),
                    height: Val::Px(24.0),
                    ..default()
                },
                skin_piece(
                    system,
                    Rect::from_corners(
                        Vec2::new(origin + x as f32 * 24.0, y as f32 * 24.0),
                        Vec2::new(origin + 8.0 + x as f32 * 24.0, 8.0 + y as f32 * 24.0),
                    ),
                ),
            ));
        }
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                right: Val::Px(24.0),
                top: if x == 0 { Val::Px(0.0) } else { Val::Auto },
                bottom: if x == 1 { Val::Px(0.0) } else { Val::Auto },
                height: Val::Px(24.0),
                ..default()
            },
            skin_piece(
                system,
                Rect::new(
                    origin + 8.0,
                    x as f32 * 24.0,
                    origin + 24.0,
                    x as f32 * 24.0 + 8.0,
                ),
            ),
        ));
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(24.0),
                bottom: Val::Px(24.0),
                left: if x == 0 { Val::Px(0.0) } else { Val::Auto },
                right: if x == 1 { Val::Px(0.0) } else { Val::Auto },
                width: Val::Px(24.0),
                ..default()
            },
            skin_piece(
                system,
                Rect::new(
                    origin + x as f32 * 24.0,
                    8.0,
                    origin + 8.0 + x as f32 * 24.0,
                    24.0,
                ),
            ),
        ));
    }
}
