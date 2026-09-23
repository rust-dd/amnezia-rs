use super::{Phase, Screen, ShopState, logic};
use crate::font::bitmap::{BitmapFont, PixelText};
use crate::gamedata::GameData;
use crate::state::{Inventory, Party};
use crate::terms::Terms;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

mod layout;
pub(super) mod party;
pub(in crate::shop) mod pixels;
#[cfg(test)]
mod tests;
mod text;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::shop) enum Window {
    Help,
    Buy,
    Sell,
    Number,
    Party,
    Status,
    Gold,
    Message,
    EmptyCenter,
    EmptyLeft,
}

#[derive(Component, Clone, Copy)]
pub(in crate::shop) enum Part {
    Root,
    Window(Window),
    Text(Window),
    Cursor(Window),
    Arrow(Window, bool),
    Face,
    Character(usize),
    Indicator(usize),
}

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

fn visible(show: bool) -> Visibility {
    if show {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

pub(super) fn spawn_ui(mut commands: Commands, server: Res<AssetServer>) {
    layout::spawn(&mut commands, &server.load("graphics/System/System.png"));
}

#[derive(SystemParam)]
pub(super) struct Drawing<'w, 's> {
    parts: Query<
        'w,
        's,
        (
            &'static Part,
            &'static mut Node,
            &'static mut Visibility,
            Option<&'static Children>,
        ),
    >,
    texts: Query<'w, 's, (&'static Part, &'static mut PixelText)>,
    images: Query<'w, 's, &'static mut ImageNode>,
}

#[derive(SystemParam)]
pub(super) struct Presentation<'w> {
    screen: Res<'w, Screen>,
    data: Res<'w, GameData>,
    inventory: Res<'w, Inventory>,
    terms: Res<'w, Terms>,
    font: Res<'w, BitmapFont>,
    dialogue: Res<'w, crate::dialogue::Dialogue>,
    party: Res<'w, Party>,
    equipment: Res<'w, crate::equipment::Equipment>,
    server: Res<'w, AssetServer>,
}

pub(super) fn update_ui(p: Presentation, mut drawing: Drawing) {
    for (part, _, mut visibility, _) in &mut drawing.parts {
        if matches!(part, Part::Root) {
            *visibility = visible(matches!(*p.screen, Screen::Shop(_)));
        }
    }
    let Screen::Shop(state) = &*p.screen else {
        return;
    };
    let face = p.dialogue.face.graphic();
    for (part, mut node, mut visibility, children) in &mut drawing.parts {
        match *part {
            Part::Window(window) => *visibility = visible(layout::shown(window, &state.phase)),
            Part::Text(window @ (Window::Buy | Window::Sell)) => {
                let (offset, height) = if window == Window::Buy {
                    (
                        state.scene.buy.offset,
                        logic::buyable_ids(&p.data, &state.items).len().max(7) * 16,
                    )
                } else {
                    (
                        state.scene.sell.offset,
                        logic::sell_ids(&p.data, &p.inventory)
                            .len()
                            .div_ceil(2)
                            .max(7)
                            * 16,
                    )
                };
                node.top = Val::Px(-offset as f32 * 3.0);
                node.height = Val::Px(height as f32 * 3.0);
            }
            Part::Cursor(window) => {
                let (x, y, width, phase) = match window {
                    Window::Buy => (
                        4,
                        8 + state.scene.buy.cursor_y,
                        176,
                        state.scene.buy.cursor_frame,
                    ),
                    Window::Sell => (
                        4 + state.scene.sell.cursor_index as i32 % 2 * 160,
                        8 + state.scene.sell.cursor_y,
                        152,
                        state.scene.sell.cursor_frame,
                    ),
                    Window::Number => (154, 40, 20, state.scene.number_frame),
                    Window::Message => {
                        let Phase::Command { cursor, .. } = state.phase else {
                            *visibility = Visibility::Hidden;
                            continue;
                        };
                        let indent = if face.is_some() { 72 } else { 0 };
                        (
                            12 + indent,
                            24 + cursor as i32 * 16,
                            296 - indent as u32,
                            state.scene.command_frame,
                        )
                    }
                    _ => unreachable!(),
                };
                *visibility = Visibility::Inherited;
                node.left = Val::Px(x as f32 * 3.0);
                node.top = Val::Px(y as f32 * 3.0);
                node.width = Val::Px(width as f32 * 3.0);
                for child in children.into_iter().flatten() {
                    if let Ok(mut image) = drawing.images.get_mut(*child) {
                        crate::windowskin::cursor_phase(
                            &mut image,
                            if phase <= 10 { 64.0 } else { 96.0 },
                        );
                    }
                }
            }
            Part::Arrow(window, up) => {
                let arrows = if window == Window::Buy {
                    state.scene.buy.arrows
                } else {
                    state.scene.sell.arrows
                };
                *visibility = visible(arrows[usize::from(!up)]);
            }
            Part::Face => *visibility = visible(face.is_some()),
            _ => {}
        }
    }
    let equipped = p
        .party
        .snapshot()
        .iter()
        .filter_map(|id| p.data.actor(*id))
        .flat_map(|actor| p.equipment.slots(actor))
        .filter(|id| *id != 0 && *id == state.scene.item_id)
        .count();
    for (part, mut label) in &mut drawing.texts {
        let Part::Text(window) = part else { continue };
        let content = match window {
            Window::Help => text::help(state.scene.help_id, &p.data),
            Window::Buy => text::entries(
                &logic::buyable_ids(&p.data, &state.items),
                true,
                &p.data,
                &p.inventory,
                &p.font,
            ),
            Window::Sell => text::entries(
                &logic::sell_ids(&p.data, &p.inventory),
                false,
                &p.data,
                &p.inventory,
                &p.font,
            ),
            Window::Number => match &state.phase {
                Phase::Number(number) => text::quantity(number, &p.data, &p.terms, &p.font),
                _ => PixelText::default(),
            },
            Window::Gold => text::gold(p.inventory.gold(), &p.terms, &p.font),
            Window::Status => text::status(
                p.inventory.count(state.scene.item_id),
                equipped,
                &p.terms,
                &p.font,
            ),
            Window::Message => text::message(state, face.is_some(), &p.terms),
            _ => unreachable!(),
        };
        label.set_if_neq(content);
    }
    if let Some((name, index)) = face {
        for (part, _, _, children) in &drawing.parts {
            if matches!(part, Part::Face) {
                for child in children.into_iter().flatten() {
                    if let Ok(mut image) = drawing.images.get_mut(*child) {
                        image.image = p.server.load(crate::assets::resolve_png("FaceSet", name));
                        image.rect = Some(Rect::new(
                            (index % 4 * 48) as f32,
                            (index / 4 * 48) as f32,
                            (index % 4 * 48 + 48) as f32,
                            (index / 4 * 48 + 48) as f32,
                        ));
                    }
                }
            }
        }
    }
}
