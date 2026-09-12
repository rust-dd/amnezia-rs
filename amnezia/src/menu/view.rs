//! Field-menu windows use native coordinates scaled threefold for Bevy UI.
//! Subscreens still share a content panel; their state and input live in the parent.

use crate::assets::resolve_png;
use crate::equipment::Equipment;
use crate::font::GameFont;
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::terms::Terms;
use crate::vitals::Vitals;
use crate::windowskin::frame;
use bevy::prelude::*;
use bevy::text::{FontSource, Justify, LineHeight, TextLayout};

use super::{MenuOpen, MenuScreen, MenuState, command, render};

pub(crate) mod smoke;
#[cfg(test)]
mod tests;

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

/// The three windowskin selection cursors: the command list, the status
/// member-select, and the scrolling content lists (item/skill).
#[derive(Clone, Copy)]
enum CursorId {
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

/// Spawn the initially hidden overlay and its four windows.
pub(super) fn spawn_ui(
    mut commands: Commands,
    font: Res<GameFont>,
    asset_server: Res<AssetServer>,
) {
    let system: Handle<Image> = asset_server.load("graphics/System/System.png");
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
            GlobalZIndex(100),
            MenuWindow(WindowId::Panel),
        ))
        .with_children(|panel| {
            spawn_command_window(panel, &system, &font);
            spawn_gold_window(panel, &system, &font);
            spawn_status_window(panel, &system, &font);
            spawn_content_window(panel, &system, &font);
        });
}

/// The command window (RM2000 0,0,88,96 → 0,0,264,288): five bare command rows
/// under a windowskin cursor.
fn spawn_command_window(panel: &mut ChildSpawnerCommands, system: &Handle<Image>, font: &GameFont) {
    panel
        .spawn((
            window_node(0.0, 0.0, 264.0, 288.0),
            Visibility::Hidden,
            MenuWindow(WindowId::Command),
        ))
        .with_children(|w| {
            frame(w, system);
            cursor_sprite(w, system, CursorId::Command, 12.0, 240.0, 48.0);
            for i in 0..command::COMMANDS.len() {
                w.spawn((
                    text_at(font, 24.0, CMD_ROW_TOP + 6.0 + i as f32 * CMD_ROW_PITCH),
                    MenuText(TextSlot::Command(i)),
                ));
            }
        });
}

/// The gold window (RM2000 0,208,88,32 → 0,624,264,96): a right-aligned
/// `<amount> <currency-term>` line (`Window_Gold` / `DrawCurrencyValue`).
fn spawn_gold_window(panel: &mut ChildSpawnerCommands, system: &Handle<Image>, font: &GameFont) {
    panel
        .spawn((
            window_node(0.0, 624.0, 264.0, 96.0),
            Visibility::Hidden,
            MenuWindow(WindowId::Gold),
        ))
        .with_children(|w| {
            frame(w, system);
            w.spawn((
                Text::new(String::new()),
                text_font(font),
                TextColor(Color::WHITE),
                TextLayout::justify(Justify::Right),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(24.0),
                    right: Val::Px(24.0),
                    top: Val::Px(30.0),
                    ..default()
                },
                MenuText(TextSlot::Gold),
            ));
        });
}

/// The status window (RM2000 88,0,232,240 → 264,0,696,720): a portrait plus the
/// member's identity and vitals per row, under the member-select cursor. Field
/// offsets are `Window_MenuStatus::Refresh` values ×3.
fn spawn_status_window(panel: &mut ChildSpawnerCommands, system: &Handle<Image>, font: &GameFont) {
    panel
        .spawn((
            window_node(264.0, 0.0, 696.0, 720.0),
            Visibility::Hidden,
            MenuWindow(WindowId::Status),
        ))
        .with_children(|w| {
            frame(w, system);
            cursor_sprite(w, system, CursorId::Status, 180.0, 504.0, 144.0);
            w.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Px(24.0),
                right: Val::Px(24.0),
                bottom: Val::Px(24.0),
                overflow: Overflow::clip(),
                ..default()
            })
            .with_children(|w| {
                for slot in 0..MAX_SLOTS {
                    let top = slot as f32 * MEMBER_PITCH;
                    w.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(top),
                            width: Val::Px(144.0),
                            height: Val::Px(144.0),
                            ..default()
                        },
                        ImageNode::default(),
                        Visibility::Hidden,
                        MenuFace(slot),
                    ));
                    let field = |f, x, y| {
                        (
                            text_at(font, x, y),
                            MenuText(TextSlot::Member { slot, field: f }),
                        )
                    };
                    w.spawn(field(MemberField::Name, 168.0, top + 6.0));
                    w.spawn(field(MemberField::Title, 432.0, top + 6.0));
                    w.spawn(field(MemberField::Level, 168.0, top + 54.0));
                    w.spawn(field(MemberField::Condition, 294.0, top + 54.0));
                    w.spawn(field(MemberField::Hp, 486.0, top + 54.0));
                    w.spawn(field(MemberField::Exp, 168.0, top + 102.0));
                    w.spawn(field(MemberField::Sp, 486.0, top + 102.0));
                }
            });
        });
}

/// The near-fullscreen content window every sub-screen's text flows into, with a
/// windowskin cursor for the scrolling item/skill lists.
fn spawn_content_window(panel: &mut ChildSpawnerCommands, system: &Handle<Image>, font: &GameFont) {
    panel
        .spawn((
            window_node(24.0, 24.0, 912.0, 672.0),
            Visibility::Hidden,
            MenuWindow(WindowId::Content),
        ))
        .with_children(|w| {
            frame(w, system);
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

/// A blank absolutely-positioned text node at `(x, y)` in the game font.
fn text_at(font: &GameFont, x: f32, y: f32) -> impl Bundle {
    (
        Text::new(String::new()),
        text_font(font),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            ..default()
        },
    )
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
    asset_server: Res<AssetServer>,
    mut windows: Query<
        (&MenuWindow, &mut Visibility),
        (Without<MenuCursor>, Without<MenuText>, Without<MenuFace>),
    >,
    mut cursors: Query<
        (&MenuCursor, &mut Node, &mut Visibility),
        (Without<MenuWindow>, Without<MenuText>, Without<MenuFace>),
    >,
    mut texts: Query<
        (&MenuText, &mut Text, &mut Visibility, &mut TextColor),
        (Without<MenuWindow>, Without<MenuCursor>, Without<MenuFace>),
    >,
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
            WindowId::Content => !main,
        };
        *visibility = vis(shown);
    }

    let members = render::members(&hero_name, &data, &party, &progression, &vitals);
    let gold = render::gold(&inventory, &terms);
    let content = render::content(
        &hero_name,
        state.screen,
        &data,
        &party,
        &progression,
        &inventory,
        &vitals,
        &equipment,
        &terms,
    );

    for (slot, mut text, mut visibility, mut color) in &mut texts {
        *color = TextColor(Color::WHITE);
        *visibility = Visibility::Inherited;
        match slot.0 {
            TextSlot::Command(i) => {
                **text = command::label(command::COMMANDS[i], &terms);
            }
            TextSlot::Gold => **text = gold.clone(),
            TextSlot::Content => **text = content.text.clone(),
            TextSlot::Member { slot, field } => match members.get(slot) {
                Some(member) => {
                    let (value, tint) = member_field(member, field, &terms);
                    **text = value;
                    *color = TextColor(tint);
                }
                None => {
                    **text = String::new();
                    *visibility = Visibility::Hidden;
                }
            },
        }
    }

    for (face, mut image, mut visibility) in &mut faces {
        match members.get(face.0) {
            Some(member) if !member.face_name.is_empty() => {
                image.image = asset_server.load(resolve_png("FaceSet", &member.face_name));
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
            (CursorId::Command, MenuScreen::Command) => {
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

/// The text and RM2000 font tint for one status field. The Lv / HP / SP prefixes
/// come from the real RM2000 short terms (`lvl_short` / `hp_short` / `sp_short`),
/// falling back to the abbreviations when blank. HP/SP low-value tints mirror
/// `Window_Base::GetValueFontColor` (knockout when empty, critical at ≤¼).
fn member_field(member: &render::MemberView, field: MemberField, terms: &Terms) -> (String, Color) {
    let t = &terms.0;
    match field {
        MemberField::Name => (member.name.clone(), Color::WHITE),
        MemberField::Title => (member.title.clone(), Color::WHITE),
        MemberField::Level => (
            format!("{} {}", terms.label(&t.lvl_short, "Lv"), member.level),
            Color::WHITE,
        ),
        MemberField::Condition => (member.condition.clone(), Color::WHITE),
        MemberField::Hp => (
            format!(
                "{} {}/{}",
                terms.label(&t.hp_short, "HP"),
                member.hp,
                member.max_hp
            ),
            value_color(member.hp, member.max_hp, true),
        ),
        MemberField::Exp => (member.exp.clone(), Color::WHITE),
        MemberField::Sp => (
            format!(
                "{} {}/{}",
                terms.label(&t.sp_short, "SP"),
                member.sp,
                member.max_sp
            ),
            value_color(member.sp, member.max_sp, false),
        ),
    }
}

/// `Window_Base::GetValueFontColor`: knockout red at zero (HP only), critical
/// amber at a quarter or less of the maximum, otherwise the default white.
fn value_color(have: i32, max: i32, can_knockout: bool) -> Color {
    if can_knockout && have == 0 {
        Color::srgb(0.86, 0.22, 0.22)
    } else if max > 0 && have <= max / 4 {
        Color::srgb(1.0, 0.84, 0.25)
    } else {
        Color::WHITE
    }
}

fn vis(shown: bool) -> Visibility {
    if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}
