use super::*;

pub(super) const WINDOWS: [(Window, i32, i32, u32, u32); 10] = [
    (Window::Help, 0, 0, 320, 32),
    (Window::Buy, 0, 32, 184, 128),
    (Window::Sell, 0, 32, 320, 128),
    (Window::Number, 0, 32, 184, 128),
    (Window::Party, 184, 32, 136, 48),
    (Window::Status, 184, 80, 136, 48),
    (Window::Gold, 184, 128, 136, 32),
    (Window::Message, 0, 160, 320, 80),
    (Window::EmptyCenter, 0, 32, 320, 128),
    (Window::EmptyLeft, 0, 32, 184, 128),
];

pub(super) fn shown(window: Window, phase: &Phase) -> bool {
    match window {
        Window::Help | Window::Message => true,
        Window::EmptyCenter => matches!(phase, Phase::Command { .. }),
        Window::EmptyLeft => matches!(phase, Phase::Bought { .. } | Phase::Sold { .. }),
        Window::Buy => matches!(phase, Phase::Buy { .. }),
        Window::Sell => matches!(phase, Phase::Sell { .. }),
        Window::Number => matches!(phase, Phase::Number(_)),
        Window::Party | Window::Status | Window::Gold => matches!(
            phase,
            Phase::Buy { .. } | Phase::Number(_) | Phase::Bought { .. } | Phase::Sold { .. }
        ),
    }
}

pub(super) fn spawn(commands: &mut Commands, skin: &Handle<Image>) {
    commands
        .spawn((
            Part::Root,
            node(0, 0, 320, 240),
            Visibility::Hidden,
            GlobalZIndex(110),
        ))
        .with_children(|root| {
            for (id, x, y, width, height) in WINDOWS {
                root.spawn((Part::Window(id), node(x, y, width, height)))
                    .with_children(|window| {
                        crate::windowskin::fixed_frame(window, skin, UVec2::new(width, height));
                        let cursor = match id {
                            Window::Buy => Some((4, 8, 176)),
                            Window::Sell => Some((4, 8, 152)),
                            Window::Number => Some((154, 40, 20)),
                            Window::Message => Some((12, 24, 296)),
                            _ => None,
                        };
                        if let Some((x, y, width)) = cursor {
                            window
                                .spawn((Part::Cursor(id), node(x, y, width, 16)))
                                .with_children(|cursor| crate::windowskin::cursor(cursor, skin));
                        }
                        if !matches!(id, Window::Party | Window::EmptyCenter | Window::EmptyLeft) {
                            window
                                .spawn(Node {
                                    overflow: Overflow::clip(),
                                    ..node(8, 8, width - 16, height - 16)
                                })
                                .with_children(|contents| {
                                    contents.spawn((
                                        Part::Text(id),
                                        node(0, 0, width - 16, height - 16),
                                        PixelText::default(),
                                    ));
                                });
                        }
                        if matches!(id, Window::Buy | Window::Sell) {
                            for up in [true, false] {
                                let sy = if up { 8.0 } else { 16.0 };
                                window.spawn((
                                    Part::Arrow(id, up),
                                    node((width / 2 - 8) as i32, if up { 0 } else { 120 }, 16, 8),
                                    ImageNode {
                                        image: skin.clone(),
                                        rect: Some(Rect::new(40.0, sy, 56.0, sy + 8.0)),
                                        ..default()
                                    },
                                ));
                            }
                        }
                        if id == Window::Message {
                            window
                                .spawn((Part::Face, node(16, 16, 48, 48)))
                                .with_children(|face| {
                                    face.spawn((node(0, 0, 48, 48), ImageNode::default()));
                                });
                        }
                        if id == Window::Party {
                            window
                                .spawn(Node {
                                    overflow: Overflow::clip(),
                                    ..node(4, 4, 128, 32)
                                })
                                .with_children(|party| {
                                    for member in 0..4 {
                                        party.spawn((
                                            Part::Character(member),
                                            node(member as i32 * 32, 0, 24, 32),
                                            ImageNode::default(),
                                        ));
                                        party.spawn((
                                            Part::Indicator(member),
                                            node(member as i32 * 32 + 20, 24, 8, 8),
                                            ImageNode {
                                                image: skin.clone(),
                                                ..default()
                                            },
                                        ));
                                    }
                                });
                        }
                    });
            }
        });
}
