use super::*;
use crate::font::bitmap::{BitmapFont, DEFAULT, DISABLED, PixelText, Run};
use crate::gamedata::GameData;
use crate::state::Inventory;
use crate::terms::Terms;
use crate::windowskin::fixed_frame;

#[cfg(test)]
mod tests;

#[derive(Component)]
pub(super) struct RowSlot(Panel, usize);

#[derive(Component)]
pub(super) struct Cursor(pub(super) Panel);

pub(super) fn spawn(
    mut commands: Commands,
    assets: Res<AssetServer>,
    camera: Query<Entity, With<crate::battle::HudCamera>>,
) {
    let Ok(camera) = camera.single() else { return };
    let system = assets.load("graphics/System/System.png");
    for panel in Panel::ALL {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    overflow: Overflow::clip(),
                    ..default()
                },
                panel,
                GlobalZIndex(panel as i32),
                Visibility::Hidden,
                UiTargetCamera(camera),
            ))
            .with_children(|parent| {
                fixed_frame(parent, &system, panel.size());
                parent
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            height: Val::Px(48.0),
                            ..default()
                        },
                        Cursor(panel),
                        Visibility::Hidden,
                    ))
                    .with_children(|parent| crate::windowskin::cursor(parent, &system));
                let margin = if panel == Panel::Status { 4 } else { 8 };
                parent
                    .spawn(Node {
                        position_type: PositionType::Absolute,
                        left: px(margin * 3),
                        right: px(margin * 3),
                        top: px(24),
                        bottom: px(24),
                        overflow: Overflow::clip(),
                        ..default()
                    })
                    .with_children(|contents| {
                        for slot in 0..10 {
                            contents.spawn((
                                PixelText::default(),
                                Node {
                                    position_type: PositionType::Absolute,
                                    height: Val::Px(48.0),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                RowSlot(panel, slot),
                            ));
                        }
                    });
                arrows::spawn(parent, &system, panel);
            });
    }
}

pub(super) fn panels(
    battle: Res<Battle>,
    windows: Res<motion::CommandWindows>,
    mut panels: Query<(&Panel, &mut Node, &mut Visibility)>,
) {
    if !battle.is_changed() && !windows.is_changed() {
        return;
    }
    for (panel, mut node, mut visible) in &mut panels {
        let Some((x, y, w, h)) = layout::rectangle(*panel, &battle) else {
            *visible = Visibility::Hidden;
            continue;
        };
        *visible = Visibility::Inherited;
        let x = windows.x(*panel).unwrap_or(x);
        node.left = Val::Percent(x / 320.0 * 100.0);
        node.top = Val::Percent(y / 240.0 * 100.0);
        node.width = Val::Percent(w / 320.0 * 100.0);
        node.height = Val::Percent(h / 240.0 * 100.0);
    }
}

pub(super) fn rows(
    battle: Res<Battle>,
    windows: Res<navigation::Windows>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    terms: Res<Terms>,
    font: Res<BitmapFont>,
    mut rows: Query<(&RowSlot, &mut PixelText, &mut Node, &mut Visibility)>,
) {
    if !battle.is_changed() && !inventory.is_changed() && !windows.is_changed() {
        return;
    }
    for panel in Panel::ALL {
        let mut content = content::rows(panel, &battle, &data, &inventory, &terms);
        if panel == Panel::Help {
            content[0].text = content::description(
                &battle,
                &data,
                &inventory,
                Some(windows.help_index(&battle)),
            );
        }
        let columns = panel.columns();
        let offset = windows.get(panel).offset.max(0) as usize;
        let first = offset / 16 * columns;
        let width = layout::rectangle(panel, &battle).map_or(320.0, |r| r.2);
        for (slot, mut text, mut node, mut visibility) in &mut rows {
            if slot.0 != panel {
                continue;
            }
            let row = content.get(first + slot.1).filter(|_| slot.1 < columns * 5);
            let member = (panel == Panel::Status)
                .then(|| battle.members.get(slot.1))
                .flatten()
                .filter(|_| slot.1 < 4);
            *visibility = if row.is_some() || member.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if row.is_some() || member.is_some() {
                let margin = if panel == Panel::Status { 4.0 } else { 8.0 };
                let text_width = if panel == Panel::Message {
                    width - 20.0
                } else {
                    width / columns as f32 - margin * 2.0
                };
                let runs = if let Some(member) = member {
                    status::runs(member, &battle.states, &terms, &font)
                } else {
                    let row = row.unwrap();
                    vec![Run::new(
                        row.text.clone(),
                        0,
                        0,
                        if row.enabled { DEFAULT } else { DISABLED },
                    )]
                };
                text.set_if_neq(PixelText {
                    size: UVec2::new(text_width as u32, 16),
                    runs,
                });
                node.left = Val::Px(((slot.1 % columns) as f32 * width / columns as f32) * 3.0);
                node.top =
                    Val::Px((2.0 + (slot.1 / columns) as f32 * 16.0 - (offset % 16) as f32) * 3.0);
                node.width = Val::Px(text_width * 3.0);
            }
        }
    }
}

pub(super) fn cursors(
    motion: Res<motion::CommandWindows>,
    battle: Res<Battle>,
    windows: Res<navigation::Windows>,
    mut cursors: Query<(&Cursor, &mut Node, &mut Visibility, &Children)>,
    mut images: Query<&mut ImageNode>,
) {
    for (cursor, mut node, mut visible, children) in &mut cursors {
        let selection = if cursor.0 == Panel::Status {
            layout::rectangle(Panel::Status, &battle)
                .and_then(|_| motion.status_cursor().map(|index| (index, 1)))
        } else {
            layout::selection(cursor.0, &battle)
        };
        let list = windows.get(cursor.0);
        let Some((_, columns)) = selection.filter(|_| list.count() > 0) else {
            *visible = Visibility::Hidden;
            continue;
        };
        let width = layout::rectangle(cursor.0, &battle).unwrap().2;
        let index = list.cursor_index;
        *visible = Visibility::Inherited;
        node.left = Val::Px((4.0 + (index % columns) as f32 * width / columns as f32) * 3.0);
        node.top = Val::Px((8.0 + list.cursor_y as f32) * 3.0);
        node.width = Val::Px((width / columns as f32 - 8.0) * 3.0);
        let x = list.cursor_x();
        for child in children {
            if let Ok(mut image) = images.get_mut(*child) {
                crate::windowskin::cursor_phase(&mut image, x);
            }
        }
    }
}

pub(in crate::battle) fn verify_bounds(world: &mut World) {
    let mut count = 0;
    for (slot, text, visibility) in world
        .query::<(&RowSlot, &PixelText, &InheritedVisibility)>()
        .iter(world)
    {
        if !visibility.get() || slot.0 == Panel::Help || text.runs.iter().all(|r| r.text.is_empty())
        {
            continue;
        }
        count += 1;
        let font = world.resource::<BitmapFont>();
        for run in &text.runs {
            assert!(
                run.position.x + font.width(&run.text) <= text.size.x as i32,
                "battle row is clipped: {text:?}"
            );
        }
    }
    assert!(count > 0, "battle menu has no visible text");
}

pub(in crate::battle) fn verify_layers(world: &mut World) {
    let visible = world
        .query::<(&Panel, &InheritedVisibility)>()
        .iter(world)
        .filter(|(_, v)| v.get())
        .map(|(p, _)| *p)
        .collect::<Vec<_>>();
    let battle = world.resource::<Battle>();
    for panel in Panel::ALL {
        assert_eq!(
            visible.contains(&panel),
            layout::rectangle(panel, battle).is_some(),
            "{panel:?}"
        );
    }
    assert!(visible.contains(&Panel::Help));
    assert!(visible.contains(&Panel::Skill) || visible.contains(&Panel::Item));
}
