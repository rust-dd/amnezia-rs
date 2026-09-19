use super::{Mode, SaveFiles};
use crate::font::bitmap::{BitmapFont, PixelText, Run};
use crate::save::preview::Contents;
use crate::terms::Terms;
use crate::windowskin;
use bevy::prelude::*;

pub(super) mod pixels;
#[cfg(test)]
mod tests;
mod text;

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Row(usize);
#[derive(Component)]
struct Cursor(usize);
#[derive(Component)]
struct Face(usize, usize);
#[derive(Component)]
enum Text {
    Help,
    Slot(usize),
}
#[derive(Component)]
struct Arrow(bool);

fn node(x: i32, y: i32, width: u32, height: u32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(x as f32 * 3.0),
        top: Val::Px(y as f32 * 3.0),
        width: Val::Px(width as f32 * 3.0),
        height: Val::Px(height as f32 * 3.0),
        ..default()
    }
}

fn image(system: &Handle<Image>, rect: Rect) -> ImageNode {
    ImageNode {
        image: system.clone(),
        rect: Some(rect),
        image_mode: NodeImageMode::Stretch,
        ..default()
    }
}

fn text_node(x: i32, y: i32, width: u32, height: u32) -> impl Bundle {
    (
        node(x, y, width, height),
        PixelText {
            size: UVec2::new(width, height),
            runs: Vec::new(),
        },
    )
}

pub(super) fn spawn(mut commands: Commands, server: Res<AssetServer>) {
    let system = server.load::<Image>("graphics/System/System.png");
    commands
        .spawn((
            Root,
            node(0, 0, 320, 240),
            Visibility::Hidden,
            GlobalZIndex(110),
            image(&system, Rect::new(0.0, 32.0, 1.0, 33.0)),
        ))
        .with_children(|parent| {
            parent.spawn(node(0, 0, 320, 32)).with_children(|parent| {
                windowskin::frame(parent, &system);
                parent.spawn((Text::Help, text_node(8, 8, 304, 16)));
            });
            parent
                .spawn(Node {
                    overflow: Overflow::clip(),
                    ..node(0, 40, 320, 192)
                })
                .with_children(|parent| {
                    for index in 0..crate::save::slots::COUNT as usize {
                        parent
                            .spawn((Row(index), node(0, index as i32 * 64, 320, 64)))
                            .with_children(|parent| {
                                windowskin::frame(parent, &system);
                                parent
                                    .spawn((Cursor(index), node(4, 8, 47, 16)))
                                    .with_children(|parent| windowskin::cursor(parent, &system));
                                parent.spawn((Text::Slot(index), text_node(4, 8, 312, 48)));
                                for member in 0..4 {
                                    parent.spawn((
                                        Face(index, member),
                                        node(96 + member as i32 * 56, 8, 48, 48),
                                        ImageNode::default(),
                                    ));
                                }
                            });
                    }
                });
            for up in [true, false] {
                let source_y = if up { 8.0 } else { 16.0 };
                parent.spawn((
                    Arrow(up),
                    node(152, if up { 32 } else { 232 }, 16, 8),
                    image(&system, Rect::new(40.0, source_y, 56.0, source_y + 8.0)),
                ));
            }
        });
}

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Drawing<'w, 's> {
    nodes: Query<'w, 's, GeometryParts, FileGeometry>,
    texts: Query<'w, 's, (&'static Text, &'static mut PixelText)>,
    faces: Query<'w, 's, (Entity, &'static Face)>,
    cursors: Query<'w, 's, (&'static Cursor, &'static Children)>,
    images: Query<'w, 's, &'static mut ImageNode>,
}

type GeometryParts = (
    &'static mut Node,
    &'static mut Visibility,
    Option<&'static Root>,
    Option<&'static Row>,
    Option<&'static Cursor>,
    Option<&'static Arrow>,
    Option<&'static Face>,
);
type FileGeometry = Or<(With<Root>, With<Row>, With<Cursor>, With<Arrow>, With<Face>)>;

pub(super) fn update(
    files: Res<SaveFiles>,
    terms: Res<Terms>,
    font: Res<BitmapFont>,
    server: Res<AssetServer>,
    mut drawing: Drawing,
) {
    let entries = files.entries.as_deref().filter(|_| !files.suspended);
    let nav = &files.navigation;
    for (mut node, mut visibility, root, row, cursor, arrow, face) in &mut drawing.nodes {
        if root.is_some() {
            *visibility = if entries.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if let Some(row) = row {
            node.top = Val::Px(((row.0 as i32 - nav.top as i32) * 64 + nav.offset()) as f32 * 3.0);
        }
        let visible = if let Some(cursor) = cursor {
            node.width = Val::Px((font.width(&crate::i18n::tr(&terms.0.file)) + 23) as f32 * 3.0);
            Some(cursor.0 == nav.index)
        } else if let Some(arrow) = arrow {
            Some(
                nav.arrow < 20
                    && if arrow.0 {
                        nav.top > 0
                    } else {
                        nav.top + 3 < crate::save::slots::COUNT as usize
                    },
            )
        } else {
            face.map(|face| face_data(entries, face).is_some_and(|(name, _)| !name.is_empty()))
        };
        if let Some(visible) = visible {
            *visibility = if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    let Some(entries) = entries else {
        return;
    };
    for (kind, mut text) in &mut drawing.texts {
        let runs = match kind {
            Text::Help => vec![Run::new(
                crate::i18n::tr(if files.mode == Mode::Load {
                    &terms.0.load_game_message
                } else {
                    &terms.0.save_game_message
                }),
                0,
                2,
                0,
            )],
            Text::Slot(index) => {
                text::row(*index, &entries[*index].contents, files.mode, &terms, &font)
            }
        };
        if text.runs != runs {
            text.runs = runs;
        }
    }
    for (entity, face) in &drawing.faces {
        if let Some((name, index)) = face_data(Some(entries), face)
            && !name.is_empty()
            && let Ok(mut image) = drawing.images.get_mut(entity)
        {
            image.image = server.load(crate::assets::resolve_png("FaceSet", name));
            let x = (index % 4) as f32 * 48.0;
            let y = (index / 4) as f32 * 48.0;
            image.rect = Some(Rect::new(x, y, x + 48.0, y + 48.0));
        }
    }
    for (cursor, children) in &drawing.cursors {
        let origin = if nav.cursors[cursor.0] <= 10 {
            64.0
        } else {
            96.0
        };
        for child in children {
            if let Ok(mut image) = drawing.images.get_mut(*child) {
                windowskin::cursor_phase(&mut image, origin);
            }
        }
    }
}

fn face_data<'a>(
    entries: Option<&'a [crate::save::preview::Entry]>,
    face: &Face,
) -> Option<&'a (String, u32)> {
    match &entries?.get(face.0)?.contents {
        Contents::Party(party) => party.faces.get(face.1),
        _ => None,
    }
}
