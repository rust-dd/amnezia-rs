use super::{MenuOpen, MenuScreen, MenuState, render, vis, window_node};
use crate::equipment::Equipment;
use crate::font::bitmap::{BitmapFont, PixelText};
use crate::gamedata::GameData;
use crate::menu::skills::{self, List};
use crate::progression::Progression;
use crate::state::Party;
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
            for (part, y) in [(Part::Help, 0), (Part::Status, 32)] {
                root.spawn(node(0, y, 320, 32)).with_children(|window| {
                    crate::windowskin::fixed_frame(window, system, UVec2::new(320, 32));
                    window.spawn((part, node(8, 8, 304, 16), PixelText::default()));
                });
            }
            root.spawn(node(0, 64, 320, 176)).with_children(|list| {
                crate::windowskin::fixed_frame(list, system, UVec2::new(320, 176));
                list.spawn((Part::Cursor, node(4, 8, 152, 16)))
                    .with_children(|cursor| crate::windowskin::cursor(cursor, system));
                list.spawn(node(8, 8, 304, 160)).with_children(|content| {
                    content.spawn((Part::Entries, node(0, 0, 304, 160), PixelText::default()));
                });
                for up in [true, false] {
                    let sy = if up { 8.0 } else { 16.0 };
                    list.spawn((
                        Part::Arrow(up),
                        node(152, if up { 0 } else { 168 }, 16, 8),
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
    list: Res<List>,
    data: Res<GameData>,
    party: Res<Party>,
    progression: Res<Progression>,
    vitals: Res<Vitals>,
    equipment: Res<Equipment>,
    hero: Res<HeroName>,
    terms: Res<Terms>,
    font: Res<BitmapFont>,
    mut drawing: Drawing,
) {
    let active = open.0 && matches!(state.screen, MenuScreen::SkillList { .. });
    let nav = &list.navigation;
    for (part, mut node, mut visibility, children) in &mut drawing.parts {
        match part {
            Part::Root => *visibility = vis(active),
            Part::Entries => node.top = Val::Px(-nav.offset as f32 * 3.0),
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
            Part::Help | Part::Status => {}
        }
    }
    if !active {
        return;
    }
    let MenuScreen::SkillList { member, .. } = state.screen else {
        return;
    };
    let members = render::members(&hero, &data, &party, &progression, &vitals);
    for (part, mut text) in &mut drawing.texts {
        let next = match part {
            Part::Help => text::help(member, nav.help_index, &data, &party, &progression),
            Part::Status => members
                .get(member)
                .map(|member| status(member, &terms, &font))
                .unwrap_or_default(),
            Part::Entries => {
                text::entries(member, &data, &party, &progression, &vitals, &equipment)
            }
            _ => continue,
        };
        if *text != next {
            *text = next;
        }
    }
    let height = skills::known_skills(member, &data, &party, &progression)
        .len()
        .div_ceil(2)
        .max(10) as f32
        * 48.0;
    for (part, mut node, _, _) in &mut drawing.parts {
        if matches!(part, Part::Entries) {
            node.height = Val::Px(height);
        }
    }
}

#[cfg(test)]
mod tests;
