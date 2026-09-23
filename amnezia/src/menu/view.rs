//! Field-menu windows use native coordinates scaled threefold for Bevy UI.

use crate::assets::resolve_png;
use crate::equipment::Equipment;
use crate::font::GameFont;
use crate::font::bitmap::{PixelText, Run};
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::terms::Terms;
use crate::vitals::Vitals;
use crate::windowskin::fixed_frame;
use bevy::prelude::*;
use bevy::text::{FontSource, LineHeight, TextLayout};

use super::{MenuOpen, MenuScreen, MenuState, command, render};

#[cfg(test)]
mod cell_tests;
pub(super) mod clocks;
pub(super) mod end_game;
pub(super) mod equipment;
pub(super) mod item_list;
mod main_pixels;
mod main_text;
mod main_windows;
pub(super) mod skill_list;
pub(crate) mod smoke;
pub(super) mod target;
#[cfg(test)]
mod tests;
pub(crate) mod text_smoke;

/// One RM2000 FaceSet cell is 48×48 in a 4×4 grid; drawn at native size in the
/// 144×144 (48×3) portrait box the status layout reserves.
const FACE_CELL: f32 = 48.0;

/// The most party members the status window lays out (RM2000 caps the party at 4).
const MAX_SLOTS: usize = 4;

/// Command-list row pitch (RM2000 16 × 3) and the top of the first row/cursor.
const CMD_ROW_PITCH: f32 = 48.0;
const CMD_ROW_TOP: f32 = 24.0;

/// Status-window per-member row pitch (RM2000 58 × 3) and the top of the first
/// member row (past the 8px windowskin border).
const MEMBER_PITCH: f32 = 174.0;
const MEMBER_TOP: f32 = 24.0;

/// Content-list row pitch: the fixed line height composed text is laid out with,
/// so the content cursor lands on the right row, and the text's top inset.
const CONTENT_LINE: f32 = crate::font::UI_LINE_PX;
const CONTENT_TOP: f32 = 16.0;

/// The whole-screen overlay; per-window nodes toggle inside it by screen.
#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowId {
    Panel,
    Command,
    Gold,
    Status,
    Content,
}

#[derive(Component)]
pub(super) struct MenuWindow(WindowId);

/// Cursors for the main windows and the legacy content panel.
#[derive(Clone, Copy)]
pub(in crate::menu) enum CursorId {
    Command,
    Status,
    Content,
}

#[derive(Component)]
pub(super) struct MenuCursor(CursorId);

/// One field of a status-window member row, at its `Window_MenuStatus` offset.
#[derive(Clone, Copy)]
enum MemberField {
    Name,
    Title,
    Level,
    Condition,
    Hp,
    Exp,
    Sp,
}

/// Which text a node carries, so one query fills them all.
#[derive(Clone, Copy)]
enum TextSlot {
    Command(usize),
    Gold,
    Content,
    Member { slot: usize, field: MemberField },
}

#[derive(Component)]
pub(super) struct MenuText(TextSlot);

/// A status-window portrait for member slot `0`-based.
#[derive(Component)]
pub(super) struct MenuFace(usize);

/// Spawn the initially hidden field-menu overlay and its scene windows.
pub(super) fn spawn_ui(
    mut commands: Commands,
    font: Res<GameFont>,
    asset_server: Res<AssetServer>,
) {
    let system = asset_server.load::<Image>("graphics/System/System.png");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            },
            Visibility::Hidden,
            ImageNode {
                image: system.clone(),
                rect: Some(Rect::new(0.0, 32.0, 1.0, 33.0)),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            GlobalZIndex(100),
            MenuWindow(WindowId::Panel),
        ))
        .with_children(|panel| {
            main_windows::spawn(panel, &system);
            spawn_content_window(panel, &system, &font);
            item_list::spawn(panel, &system);
            skill_list::spawn(panel, &system);
            equipment::spawn(panel, &system);
            target::spawn(panel, &system);
            end_game::spawn(panel, &system);
        });
}

/// The remaining text-based subscreens share this content panel and cursor.
fn spawn_content_window(panel: &mut ChildSpawnerCommands, system: &Handle<Image>, font: &GameFont) {
    panel
        .spawn((
            window_node(24.0, 24.0, 912.0, 672.0),
            Visibility::Hidden,
            MenuWindow(WindowId::Content),
        ))
        .with_children(|w| {
            fixed_frame(w, system, UVec2::new(304, 224));
            cursor_sprite(w, system, CursorId::Content, 10.0, 884.0, CONTENT_LINE);
            w.spawn((
                Text::new(String::new()),
                text_font(font),
                TextColor(Color::WHITE),
                LineHeight::Px(CONTENT_LINE),
                TextLayout::no_wrap(),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0),
                    top: Val::Px(CONTENT_TOP),
                    right: Val::Px(16.0),
                    bottom: Val::Px(16.0),
                    ..default()
                },
                MenuText(TextSlot::Content),
            ));
        });
}

/// An absolutely-positioned window box at `(left, top)` sized `width`×`height`,
/// clipping its content.
fn window_node(left: f32, top: f32, width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(left),
        top: Val::Px(top),
        width: Val::Px(width),
        height: Val::Px(height),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// Cursor corners retain their eight native pixels even on a sixteen-pixel row.
fn cursor_sprite(
    window: &mut ChildSpawnerCommands,
    system: &Handle<Image>,
    id: CursorId,
    left: f32,
    width: f32,
    height: f32,
) {
    window
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(0.0),
                width: Val::Px(width),
                height: Val::Px(height),
                ..default()
            },
            Visibility::Hidden,
            MenuCursor(id),
        ))
        .with_children(|parent| crate::windowskin::cursor(parent, system));
}

/// The native 12px font cell at the UI's threefold coordinate scale.
fn text_font(font: &GameFont) -> TextFont {
    TextFont {
        font: FontSource::Handle(font.0.clone()),
        font_size: FontSize::Px(crate::font::UI_FONT_PX),
        ..default()
    }
}

/// Reflect the menu state into the four windows: show the overlay, pick the main
/// (command/gold/status) or content window set for the active screen, fill every
/// text and portrait, and place each windowskin cursor.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn update_ui(
    open: Res<MenuOpen>,
    state: Res<MenuState>,
    data: Res<GameData>,
    party: Res<Party>,
    progression: Res<Progression>,
    inventory: Res<Inventory>,
    vitals: Res<Vitals>,
    equipment: Res<Equipment>,
    terms: Res<Terms>,
    hero_name: Res<crate::text::HeroName>,
    drawing: main_text::Drawing,
    mut windows: Query<
        (&MenuWindow, &mut Visibility),
        (Without<MenuCursor>, Without<MenuText>, Without<MenuFace>),
    >,
    mut cursors: Query<
        (&MenuCursor, &mut Node, &mut Visibility),
        (Without<MenuWindow>, Without<MenuText>, Without<MenuFace>),
    >,
    mut texts: Query<
        (&MenuText, &mut PixelText, &mut Visibility, &mut Node),
        (Without<MenuWindow>, Without<MenuCursor>, Without<MenuFace>),
    >,
    mut content_texts: Query<&mut Text, With<MenuText>>,
    mut faces: Query<
        (&MenuFace, &mut ImageNode, &mut Visibility),
        (Without<MenuWindow>, Without<MenuCursor>, Without<MenuText>),
    >,
) {
    if !open.is_changed()
        && !state.is_changed()
        && !inventory.is_changed()
        && !vitals.is_changed()
        && !equipment.is_changed()
        && !hero_name.is_changed()
        && !party.is_changed()
        && !progression.is_changed()
        && !terms.is_changed()
        && !drawing.font.is_changed()
        && !drawing.save_access.is_changed()
    {
        return;
    }
    let main = matches!(
        state.screen,
        MenuScreen::Command | MenuScreen::MemberSelect { .. }
    );
    for (window, mut visibility) in &mut windows {
        let shown = match window.0 {
            WindowId::Panel => open.0,
            WindowId::Command | WindowId::Gold | WindowId::Status => main,
            WindowId::Content => {
                !main
                    && !matches!(
                        state.screen,
                        MenuScreen::EndGame { .. }
                            | MenuScreen::ItemList { .. }
                            | MenuScreen::ItemTarget { .. }
                            | MenuScreen::SkillList { .. }
                            | MenuScreen::SkillTarget { .. }
                            | MenuScreen::Equip { .. }
                    )
            }
        };
        *visibility = vis(shown);
    }

    let members = render::members(&hero_name, &data, &party, &progression, &vitals);
    let content = render::content(
        &hero_name,
        state.screen,
        &data,
        &party,
        &progression,
        &vitals,
        &equipment,
        &terms,
    );

    for mut text in &mut content_texts {
        **text = content.text.clone();
    }
    for (slot, mut text, mut visibility, mut node) in &mut texts {
        *visibility = Visibility::Inherited;
        let runs = match slot.0 {
            TextSlot::Command(i) => {
                vec![Run::new(
                    command::label(command::COMMANDS[i], &terms),
                    0,
                    0,
                    main_text::command_color(i, members.len(), drawing.save_access.0),
                )]
            }
            TextSlot::Gold => main_text::gold(inventory.gold(), &terms, &drawing.font),
            TextSlot::Content => continue,
            TextSlot::Member { slot, field } => match members.get(slot) {
                Some(member) => {
                    if matches!(field, MemberField::Hp | MemberField::Sp) {
                        let width = main_text::vital_width(member);
                        text.size.x = width;
                        node.width = Val::Px(width as f32 * 3.0);
                        node.left = Val::Px((216 - width) as f32 * 3.0);
                    }
                    main_text::member(member, field, &terms, &drawing.font)
                }
                None => {
                    *visibility = Visibility::Hidden;
                    Vec::new()
                }
            },
        };
        if text.runs != runs {
            text.runs = runs;
        }
    }

    for (face, mut image, mut visibility) in &mut faces {
        match members.get(face.0) {
            Some(member) if !member.face_name.is_empty() => {
                image.image = drawing
                    .server
                    .load(resolve_png("FaceSet", &member.face_name));
                let (col, row) = (
                    (member.face_index % 4) as f32,
                    (member.face_index / 4) as f32,
                );
                image.rect = Some(Rect::new(
                    col * FACE_CELL,
                    row * FACE_CELL,
                    col * FACE_CELL + FACE_CELL,
                    row * FACE_CELL + FACE_CELL,
                ));
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
    }

    for (cursor, mut node, mut visibility) in &mut cursors {
        let top = match (cursor.0, state.screen) {
            (CursorId::Command, MenuScreen::Command | MenuScreen::MemberSelect { .. }) => {
                Some(CMD_ROW_TOP + state.cursor as f32 * CMD_ROW_PITCH)
            }
            (CursorId::Status, MenuScreen::MemberSelect { cursor, .. }) => {
                Some(MEMBER_TOP + cursor as f32 * MEMBER_PITCH)
            }
            (CursorId::Content, _) => content
                .cursor_line
                .map(|line| CONTENT_TOP + line as f32 * CONTENT_LINE),
            _ => None,
        };
        match top {
            Some(y) => {
                node.top = Val::Px(y);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

fn vis(shown: bool) -> Visibility {
    if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}
