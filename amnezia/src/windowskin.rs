use bevy::prelude::*;

pub(crate) mod background;
pub(crate) mod motion;

pub(crate) fn fixed_frame(parent: &mut ChildSpawnerCommands, system: &Handle<Image>, size: UVec2) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        background::Pixels(size),
    ));
    border(parent, system, 32.0);
}

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

pub(crate) fn cursor_phase(image: &mut ImageNode, origin: f32) {
    if let Some(rect) = &mut image.rect {
        let offset = origin - (rect.min.x / 32.0).floor() * 32.0;
        rect.min.x += offset;
        rect.max.x += offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, |mut commands: Commands| {
                commands
                    .spawn(Node::default())
                    .with_children(|parent| cursor(parent, &Handle::default()));
            });
        app.update();
        app
    }

    #[test]
    fn cursor_parts_keep_eight_native_pixel_corners() {
        let mut app = app();
        let world = app.world_mut();
        let pieces = world
            .query::<(&Node, &ImageNode)>()
            .iter(world)
            .collect::<Vec<_>>();
        assert_eq!(pieces.len(), 9);
        let corners = pieces
            .iter()
            .filter(|(_, image)| image.rect.unwrap().size() == Vec2::splat(8.0))
            .collect::<Vec<_>>();
        assert_eq!(corners.len(), 4);
        for (node, _) in corners {
            assert_eq!((node.width, node.height), (Val::Px(24.0), Val::Px(24.0)));
        }
    }

    #[test]
    fn both_cursor_phases_preserve_every_piece_without_accumulated_offset() {
        let mut app = app();
        let world = app.world_mut();
        for mut image in world.query::<&mut ImageNode>().iter_mut(world) {
            let first = image.rect.unwrap();
            for _ in 0..3 {
                cursor_phase(&mut image, 96.0);
                assert_eq!(
                    image.rect.unwrap(),
                    Rect::from_corners(
                        first.min + Vec2::new(32.0, 0.0),
                        first.max + Vec2::new(32.0, 0.0)
                    )
                );
            }
            cursor_phase(&mut image, 64.0);
            assert_eq!(image.rect, Some(first));
        }
    }
}
