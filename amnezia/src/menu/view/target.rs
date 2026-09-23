use super::{MenuOpen, MenuState, render, vis, window_node};
use crate::font::bitmap::{BitmapFont, PixelText};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

mod model;
mod pixels;
pub(crate) mod smoke;
mod text;
pub(in crate::menu) use model::Clock;
pub(in crate::menu) use text::party as party_text;

#[cfg(test)]
mod tests;

#[derive(Component)]
enum Part {
    Root,
    Name,
    Value,
    Party,
    Cursor,
    Face(usize),
}

fn node(x: u32, y: u32, width: u32, height: u32) -> Node {
    window_node(
        x as f32 * 3.0,
        y as f32 * 3.0,
        width as f32 * 3.0,
        height as f32 * 3.0,
    )
}

pub(super) fn spawn(panel: &mut ChildSpawnerCommands, system: &Handle<Image>) {
    panel
        .spawn((Part::Root, node(0, 0, 320, 240), Visibility::Hidden))
        .with_children(|root| {
            for (kind, y) in [(Part::Name, 0), (Part::Value, 32)] {
                root.spawn(node(0, y, 136, 32)).with_children(|window| {
                    crate::windowskin::fixed_frame(window, system, UVec2::new(136, 32));
                    window.spawn((kind, node(8, 8, 120, 16), PixelText::default()));
                });
            }
            root.spawn(node(136, 0, 184, 240)).with_children(|window| {
                crate::windowskin::fixed_frame(window, system, UVec2::new(184, 240));
                window
                    .spawn((Part::Cursor, node(60, 8, 120, 48)))
                    .with_children(|cursor| crate::windowskin::cursor(cursor, system));
                window
                    .spawn(Node {
                        overflow: Overflow::clip(),
                        ..node(8, 8, 168, 224)
                    })
                    .with_children(|contents| {
                        for slot in 0..4 {
                            contents.spawn((
                                Part::Face(slot),
                                node(0, slot as u32 * 58, 48, 48),
                                ImageNode::default(),
                                Visibility::Hidden,
                            ));
                        }
                        contents.spawn((Part::Party, node(0, 0, 168, 224), PixelText::default()));
                    });
            });
        });
}

#[derive(SystemParam)]
pub(in crate::menu) struct Context<'w> {
    data: Res<'w, crate::gamedata::GameData>,
    party: Res<'w, crate::state::Party>,
    inventory: Res<'w, crate::state::Inventory>,
    equipment: Res<'w, crate::equipment::Equipment>,
    progression: Res<'w, crate::progression::Progression>,
    vitals: Res<'w, crate::vitals::Vitals>,
    hero: Res<'w, crate::text::HeroName>,
    terms: Res<'w, crate::terms::Terms>,
    font: Res<'w, BitmapFont>,
    server: Res<'w, AssetServer>,
}

#[derive(SystemParam)]
pub(in crate::menu) struct Drawing<'w, 's> {
    parts: Query<
        'w,
        's,
        (
            Entity,
            &'static Part,
            &'static mut Node,
            &'static mut Visibility,
            Option<&'static Children>,
        ),
    >,
    texts: Query<'w, 's, (&'static Part, &'static mut PixelText)>,
    images: Query<'w, 's, &'static mut ImageNode>,
}

#[allow(clippy::too_many_arguments)]
pub(in crate::menu) fn update(
    open: Res<MenuOpen>,
    state: Res<MenuState>,
    frames: Res<crate::timing::GameFrames>,
    pause: crate::menu::scene::Pause,
    fade: Res<crate::teleport::Fade>,
    files: Res<crate::menu::save_files::SaveFiles>,
    mut clock: ResMut<Clock>,
    context: Context,
    mut drawing: Drawing,
) {
    let selection = open
        .0
        .then(|| {
            model::Selection::new(
                state.screen,
                &context.data,
                &context.party,
                &context.inventory,
                &context.equipment,
            )
        })
        .flatten();
    clock.advance(
        frames.frame,
        selection.as_ref().map(|selection| selection.key),
        pause.paused() || fade.busy() || files.active(),
    );
    let members = selection.as_ref().map_or_else(Vec::new, |_| {
        render::members(
            &context.hero,
            &context.data,
            &context.party,
            &context.progression,
            &context.vitals,
        )
    });
    for (entity, part, mut node, mut visibility, children) in &mut drawing.parts {
        match part {
            Part::Root => *visibility = vis(selection.is_some()),
            Part::Cursor => {
                let rect = selection
                    .as_ref()
                    .and_then(|selection| selection.cursor(members.len()));
                *visibility = vis(rect.is_some());
                if let Some((y, height)) = rect {
                    node.top = Val::Px(y as f32 * 3.0);
                    node.height = Val::Px(height as f32 * 3.0);
                    if let Some(children) = children {
                        for child in children {
                            if let Ok(mut image) = drawing.images.get_mut(*child) {
                                crate::windowskin::cursor_phase(
                                    &mut image,
                                    if clock.phase <= 10 { 64.0 } else { 96.0 },
                                );
                            }
                        }
                    }
                }
            }
            Part::Face(index) => {
                let member = members
                    .get(*index)
                    .filter(|member| !member.face_name.is_empty());
                *visibility = vis(member.is_some());
                if let Some(member) = member
                    && let Ok(mut image) = drawing.images.get_mut(entity)
                {
                    image.image = context
                        .server
                        .load(crate::assets::resolve_png("FaceSet", &member.face_name));
                    let x = (member.face_index % 4 * 48) as f32;
                    let y = (member.face_index / 4 * 48) as f32;
                    image.rect = Some(Rect::new(x, y, x + 48.0, y + 48.0));
                    image.image_mode = NodeImageMode::Stretch;
                }
            }
            _ => {}
        }
    }
    let Some(selection) = selection else { return };
    for (part, mut pixels) in &mut drawing.texts {
        let next = match part {
            Part::Name => text::name(&selection),
            Part::Value => text::value(&selection, &context.terms, &context.font),
            Part::Party => party_text(&members, &context.terms, &context.font),
            _ => continue,
        };
        if *pixels != next {
            *pixels = next;
        }
    }
}
