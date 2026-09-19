use super::{MenuOpen, MenuScreen, MenuState, vis, window_node};
use crate::font::bitmap::{DEFAULT, DISABLED, PixelText, Run};
use crate::gamedata::GameData;
use crate::menu::{items::List, use_item};
use crate::state::Inventory;
use bevy::prelude::*;

pub(crate) mod pixels;

#[derive(Component)]
enum Part {
    Root,
    Help,
    Entries,
    Cursor,
    Arrow(bool),
}

fn node(x: i32, y: i32, width: u32, height: u32) -> Node {
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
            root.spawn(node(0, 0, 320, 32)).with_children(|help| {
                crate::windowskin::fixed_frame(help, system, UVec2::new(320, 32));
                help.spawn((Part::Help, node(8, 8, 304, 16), PixelText::default()));
            });
            root.spawn(node(0, 32, 320, 208)).with_children(|list| {
                crate::windowskin::fixed_frame(list, system, UVec2::new(320, 208));
                list.spawn((Part::Cursor, node(4, 8, 152, 16)))
                    .with_children(|cursor| crate::windowskin::cursor(cursor, system));
                list.spawn(Node {
                    overflow: Overflow::clip(),
                    ..node(8, 8, 304, 192)
                })
                .with_children(|content| {
                    content.spawn((Part::Entries, node(0, 0, 304, 192), PixelText::default()));
                });
                for up in [true, false] {
                    let sy = if up { 8.0 } else { 16.0 };
                    list.spawn((
                        Part::Arrow(up),
                        node(152, if up { 0 } else { 200 }, 16, 8),
                        Visibility::Hidden,
                        ImageNode {
                            image: system.clone(),
                            rect: Some(Rect::new(40.0, sy, 56.0, sy + 8.0)),
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                    ));
                }
            });
        });
}

fn entries(data: &GameData, inventory: &Inventory) -> PixelText {
    let held = use_item::held_item_ids(data, inventory);
    let mut runs = Vec::new();
    for (index, id) in held.iter().enumerate() {
        let item = data.item(*id).unwrap();
        let color = if use_item::field_usable(item) {
            DEFAULT
        } else {
            DISABLED
        };
        let x = (index % 2 * 160) as i32;
        let y = (index / 2 * 16 + 2) as i32;
        runs.push(Run::new(crate::i18n::tr(&item.name), x, y, color));
        runs.push(Run::new(
            format!(":{:>3}", inventory.count(*id)),
            x + 120,
            y,
            color,
        ));
    }
    PixelText {
        size: UVec2::new(304, held.len().div_ceil(2).max(12) as u32 * 16),
        runs,
    }
}

fn help(data: &GameData, inventory: &Inventory, index: usize) -> PixelText {
    let description = crate::menu::items::item_at(index, data, inventory)
        .and_then(|id| data.item(id))
        .map(|item| crate::i18n::tr(&item.description))
        .unwrap_or_default();
    PixelText {
        size: UVec2::new(304, 16),
        runs: vec![Run::new(description, 0, 2, DEFAULT)],
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::menu) struct Drawing<'w, 's> {
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

pub(in crate::menu) fn update(
    open: Res<MenuOpen>,
    state: Res<MenuState>,
    list: Res<List>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    mut drawing: Drawing,
) {
    let active = open.0 && matches!(state.screen, MenuScreen::ItemList { .. });
    let nav = &list.navigation;
    for (part, mut node, mut visibility, children) in &mut drawing.parts {
        match part {
            Part::Root => *visibility = vis(active),
            Part::Entries => {
                node.top = Val::Px(-nav.offset as f32 * 3.0);
                node.height = Val::Px(entries_height(&data, &inventory) as f32 * 3.0);
            }
            Part::Cursor => {
                node.left = Val::Px((4 + nav.cursor_index % 2 * 160) as f32 * 3.0);
                node.top = Val::Px((8 + nav.cursor_y) as f32 * 3.0);
                if let Some(children) = children {
                    for child in children {
                        if let Ok(mut image) = drawing.images.get_mut(*child) {
                            crate::windowskin::cursor_phase(
                                &mut image,
                                if nav.cursor_frame <= 10 { 64.0 } else { 96.0 },
                            );
                        }
                    }
                }
            }
            Part::Arrow(up) => *visibility = vis(nav.arrows[usize::from(!up)]),
            Part::Help => {}
        }
    }
    if !active {
        return;
    }
    for (part, mut text) in &mut drawing.texts {
        let next = match part {
            Part::Help => help(&data, &inventory, nav.help_index),
            Part::Entries => entries(&data, &inventory),
            _ => continue,
        };
        if *text != next {
            *text = next;
        }
    }
}

fn entries_height(data: &GameData, inventory: &Inventory) -> u32 {
    crate::menu::items::selectable(data, inventory)
        .div_ceil(2)
        .max(12) as u32
        * 16
}

#[cfg(test)]
mod tests;
