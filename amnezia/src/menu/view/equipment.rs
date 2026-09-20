use super::{MenuOpen, MenuScreen, MenuState, vis, window_node};
use crate::equipment::Equipment;
use crate::font::bitmap::{BitmapFont, PixelText};
use crate::gamedata::GameData;
use crate::menu::equip::{self, Scene};
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::terms::Terms;
use crate::text::HeroName;
use crate::vitals::Vitals;
use bevy::prelude::*;

pub(crate) mod pixels;
mod text;
pub(in crate::menu) use text::status;

#[derive(Component)]
enum Part {
    Root,
    Help,
    Status,
    Slots,
    Entries,
    SlotCursor,
    ItemCursor,
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
            for (part, x, y, width, height) in [
                (Part::Help, 0, 0, 320, 32),
                (Part::Status, 0, 32, 124, 96),
                (Part::Slots, 124, 32, 196, 96),
                (Part::Entries, 0, 128, 320, 112),
            ] {
                root.spawn(node(x, y, width, height))
                    .with_children(|window| {
                        crate::windowskin::fixed_frame(window, system, UVec2::new(width, height));
                        if matches!(part, Part::Slots | Part::Entries) {
                            let cursor = if matches!(part, Part::Slots) {
                                Part::SlotCursor
                            } else {
                                Part::ItemCursor
                            };
                            let cw = if matches!(part, Part::Slots) {
                                188
                            } else {
                                152
                            };
                            window
                                .spawn((cursor, node(4, 8, cw, 16)))
                                .with_children(|cursor| crate::windowskin::cursor(cursor, system));
                        }
                        let entries = matches!(part, Part::Entries);
                        window
                            .spawn(node(8, 8, width - 16, height - 16))
                            .with_children(|content| {
                                content.spawn((
                                    part,
                                    node(0, 0, width - 16, height - 16),
                                    PixelText::default(),
                                ));
                            });
                        if entries {
                            for up in [true, false] {
                                let sy = if up { 8.0 } else { 16.0 };
                                window.spawn((
                                    Part::Arrow(up),
                                    node(152, if up { 0 } else { 104 }, 16, 8),
                                    Visibility::Hidden,
                                    ImageNode {
                                        image: system.clone(),
                                        rect: Some(Rect::new(40.0, sy, 56.0, sy + 8.0)),
                                        image_mode: NodeImageMode::Stretch,
                                        ..default()
                                    },
                                ));
                            }
                        }
                    });
            }
        });
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

#[allow(clippy::too_many_arguments)]
pub(in crate::menu) fn update(
    open: Res<MenuOpen>,
    state: Res<MenuState>,
    scene: Res<Scene>,
    data: Res<GameData>,
    party: Res<Party>,
    progression: Res<Progression>,
    inventory: Res<Inventory>,
    vitals: Res<Vitals>,
    equipment: Res<Equipment>,
    hero: Res<HeroName>,
    terms: Res<Terms>,
    font: Res<BitmapFont>,
    mut drawing: Drawing,
) {
    let active = open.0 && matches!(state.screen, MenuScreen::Equip { .. });
    for (part, _, mut visibility, _) in &mut drawing.parts {
        if matches!(part, Part::Root) {
            *visibility = vis(active);
        }
    }
    if !active {
        return;
    }
    let MenuScreen::Equip {
        member,
        slot,
        picking,
    } = state.screen
    else {
        return;
    };
    let slot = slot.min(4);
    let nav = &scene.lists[slot];
    let ids = equip::candidates(member, slot, &data, &party, &inventory);
    let height = ids.len().div_ceil(2).max(6) as u32 * 16;
    for (part, mut node, mut visibility, children) in &mut drawing.parts {
        let phase = match part {
            Part::Entries => {
                node.top = Val::Px(-nav.offset as f32 * 3.0);
                node.height = Val::Px(height as f32 * 3.0);
                None
            }
            Part::SlotCursor => {
                node.top = Val::Px((8 + slot * 16) as f32 * 3.0);
                Some(scene.slot_frame)
            }
            Part::ItemCursor => {
                *visibility = vis(picking.is_some());
                node.left = Val::Px((4 + nav.cursor_index % 2 * 160) as f32 * 3.0);
                node.top = Val::Px((8 + nav.cursor_y) as f32 * 3.0);
                Some(nav.cursor_frame)
            }
            Part::Arrow(up) => {
                *visibility = vis(nav.arrows[usize::from(!up)]);
                None
            }
            _ => None,
        };
        if let Some(phase) = phase
            && let Some(children) = children
        {
            for child in children {
                if let Ok(mut image) = drawing.images.get_mut(*child) {
                    crate::windowskin::cursor_phase(
                        &mut image,
                        if phase <= 10 { 64.0 } else { 96.0 },
                    );
                }
            }
        }
    }
    let Some(actor) = party.snapshot().get(member).and_then(|id| data.actor(*id)) else {
        return;
    };
    let current = if scene.member == Some(member) {
        scene.current
    } else {
        equip::stats(actor, &data, &progression, &vitals, equipment.slots(actor))
    };
    let help_id = if scene.member != Some(member) {
        equipment.slots(actor)[slot]
    } else if scene.picking.is_none()
        && let Some(cursor) = picking
    {
        ids.get(cursor).copied().unwrap_or(0)
    } else {
        scene.help_id
    };
    for (part, mut text) in &mut drawing.texts {
        let next = match part {
            Part::Help => text::help(help_id, &data),
            Part::Status => status(hero.actor(actor), current, scene.preview, &terms, &font),
            Part::Slots => text::slots(actor, &data, &equipment, &terms),
            Part::Entries => text::entries(&ids, &data, &inventory),
            _ => continue,
        };
        if *text != next {
            *text = next;
        }
    }
}

#[cfg(test)]
mod tests;
