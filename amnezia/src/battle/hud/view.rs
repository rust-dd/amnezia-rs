use super::*;
use crate::font::bitmap::{BitmapFont, DEFAULT, DISABLED, PixelText, Run};
use crate::gamedata::GameData;
use crate::state::Inventory;
use crate::terms::Terms;

#[derive(Component)]
pub(super) struct RowSlot(Panel, usize);

#[derive(Component)]
pub(super) struct Cursor(Panel);

fn skin_piece(system: &Handle<Image>, rect: Rect) -> ImageNode {
    ImageNode {
        image: system.clone(),
        rect: Some(rect),
        image_mode: NodeImageMode::Stretch,
        ..default()
    }
}

fn frame(parent: &mut ChildSpawnerCommands, system: &Handle<Image>) {
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
                        Vec2::new(32.0 + x as f32 * 24.0, y as f32 * 24.0),
                        Vec2::new(40.0 + x as f32 * 24.0, 8.0 + y as f32 * 24.0),
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
                Rect::new(40.0, x as f32 * 24.0, 56.0, x as f32 * 24.0 + 8.0),
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
                Rect::new(32.0 + x as f32 * 24.0, 8.0, 40.0 + x as f32 * 24.0, 24.0),
            ),
        ));
    }
}

fn image(system: &Handle<Image>, x: f32) -> ImageNode {
    ImageNode {
        image: system.clone(),
        rect: Some(Rect::new(x, 0.0, x + 32.0, 32.0)),
        image_mode: NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::all(8.0),
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale: 3.0,
        }),
        ..default()
    }
}

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
                frame(parent, &system);
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        height: Val::Px(48.0),
                        ..default()
                    },
                    image(&system, 64.0),
                    Cursor(panel),
                    Visibility::Hidden,
                ));
                for slot in 0..8 {
                    parent.spawn((
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
    scroll: Res<ListScroll>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    terms: Res<Terms>,
    font: Res<BitmapFont>,
    mut rows: Query<(&RowSlot, &mut PixelText, &mut Node, &mut Visibility)>,
) {
    if !battle.is_changed() && !inventory.is_changed() {
        return;
    }
    for panel in Panel::ALL {
        let content = content::rows(panel, &battle, &data, &inventory, &terms);
        let columns = panel.columns();
        let first = scroll.first[panel as usize];
        let width = layout::rectangle(panel, &battle).map_or(320.0, |r| r.2);
        for (slot, mut text, mut node, mut visibility) in &mut rows {
            if slot.0 != panel {
                continue;
            }
            let row = content.get(first + slot.1).filter(|_| slot.1 < columns * 4);
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
                let text_width = width / columns as f32 - margin * 2.0;
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
                node.left =
                    Val::Px((margin + (slot.1 % columns) as f32 * width / columns as f32) * 3.0);
                node.top = Val::Px((10.0 + (slot.1 / columns) as f32 * 16.0) * 3.0);
                node.width = Val::Px(text_width * 3.0);
            }
        }
    }
}

pub(super) fn cursors(
    clocks: Res<clocks::WindowClocks>,
    windows: Res<motion::CommandWindows>,
    battle: Res<Battle>,
    scroll: Res<ListScroll>,
    mut cursors: Query<(&Cursor, &mut Node, &mut Visibility, &mut ImageNode)>,
) {
    for (cursor, mut node, mut visible, mut image) in &mut cursors {
        let selection = if cursor.0 == Panel::Status {
            layout::rectangle(Panel::Status, &battle)
                .and_then(|_| windows.status_cursor().map(|index| (index, 1)))
        } else {
            layout::selection(cursor.0, &battle)
        };
        let Some((index, columns)) = selection else {
            *visible = Visibility::Hidden;
            continue;
        };
        let width = layout::rectangle(cursor.0, &battle).unwrap().2;
        let first = scroll.first[cursor.0 as usize];
        let index = index.saturating_sub(first);
        *visible = Visibility::Inherited;
        node.left = Val::Px((4.0 + (index % columns) as f32 * width / columns as f32) * 3.0);
        node.top = Val::Px((8.0 + (index / columns) as f32 * 16.0) * 3.0);
        node.width = Val::Px((width / columns as f32 - 8.0) * 3.0);
        let x = clocks.cursor_x(cursor.0);
        image.rect = Some(Rect::new(x, 0.0, x + 32.0, 32.0));
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
