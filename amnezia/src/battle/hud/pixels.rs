use super::*;
use crate::font::bitmap::{BitmapFont, DEFAULT, DISABLED, PixelText, Run};
use crate::gamedata::GameData;
use crate::state::Inventory;
use crate::terms::Terms;
use crate::windowskin::reference::Canvas;

pub(super) fn compose(world: &World) -> Vec<(u32, u32, [u8; 4])> {
    let battle = world.resource::<Battle>();
    let lists = world.resource::<navigation::Windows>();
    let windows = world.resource::<motion::CommandWindows>();
    let data = world.resource::<GameData>();
    let inventory = world.resource::<Inventory>();
    let terms = world.resource::<Terms>();
    let font = world.resource::<BitmapFont>();
    let system = world
        .resource::<AssetServer>()
        .load("graphics/System/System.png");
    let skin = world.resource::<Assets<Image>>().get(&system).unwrap();
    let mut canvas = Canvas::new(None);
    for panel in Panel::ALL {
        let Some((x, y, width, height)) = layout::rectangle(panel, battle) else {
            continue;
        };
        let left = windows.x(panel).unwrap_or(x) as i32;
        let top = y as i32;
        let width = width as u32;
        let height = height as u32;
        canvas.clip = IRect::new(left, top, left + width as i32, top + height as i32);
        canvas.window(skin, (left, top, width, height));
        let columns = panel.columns();
        let list = lists.get(panel);
        let offset = list.offset.max(0) as usize;
        let first = offset / 16 * columns;
        let selection = if panel == Panel::Status {
            windows.status_cursor().map(|index| (index, 1))
        } else {
            layout::selection(panel, battle)
        };
        if let Some((_, count)) = selection.filter(|_| list.count() > 0) {
            let index = list.cursor_index;
            canvas.cursor(
                skin,
                (
                    left + 4 + (index % count) as i32 * width as i32 / count as i32,
                    top + 8 + list.cursor_y,
                    width / count as u32 - 8,
                    16,
                ),
                list.cursor_x() as u32,
            );
        }
        let mut rows = content::rows(panel, battle, data, inventory, terms);
        if panel == Panel::Help {
            rows[0].text =
                content::description(battle, data, inventory, Some(lists.help_index(battle)));
        }
        let margin = if panel == Panel::Status { 4 } else { 8 };
        let text_width = if panel == Panel::Message {
            width - 20
        } else {
            width / columns as u32 - margin * 2
        };
        canvas.clip = IRect::new(
            left + margin as i32,
            top + 8,
            left + width as i32 - margin as i32,
            top + height as i32 - 8,
        );
        for slot in 0..columns * 5 {
            let runs = if panel == Panel::Status {
                let Some(member) = battle.members.get(slot) else {
                    continue;
                };
                status::runs(member, &battle.states, terms, font)
            } else {
                let Some(row) = rows.get(first + slot) else {
                    continue;
                };
                vec![Run::new(
                    &row.text,
                    0,
                    0,
                    if row.enabled { DEFAULT } else { DISABLED },
                )]
            };
            let text = font.render(
                &PixelText {
                    size: UVec2::new(text_width, 16),
                    runs,
                },
                skin,
            );
            canvas.blit(
                &text,
                (
                    left + margin as i32 + (slot % columns) as i32 * width as i32 / columns as i32,
                    top + 10 + (slot / columns) as i32 * 16 - (offset % 16) as i32,
                ),
                (0, 0, text_width, 16),
            );
        }
        canvas.clip = IRect::new(left, top, left + width as i32, top + height as i32);
        if !matches!(panel, Panel::Status | Panel::Help | Panel::Message) {
            for (up, visible) in [true, false].into_iter().zip(list.arrows) {
                if visible {
                    canvas.blit(
                        skin,
                        (
                            left + width as i32 / 2 - 8,
                            top + if up { 0 } else { height as i32 - 8 },
                        ),
                        (40, if up { 8 } else { 16 }, 16, 8),
                    );
                }
            }
        }
    }
    canvas.pixels()
}
