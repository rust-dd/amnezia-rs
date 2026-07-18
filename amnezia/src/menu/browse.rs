//! The read-only three-tab browse view: the party roster, the held items, and
//! the known skills, plus the scrolling viewport that keeps the row cursor in
//! sight. The interactive sub-screens live in sibling modules; this one only
//! builds the rows and composes the browse panel text.

use crate::gamedata::GameData;
use crate::i18n;
use crate::state::{Inventory, Party};

/// The tab titles, in order; the index into this is the menu's `tab`.
pub(super) const TABS: [&str; 3] = ["Party", "Items", "Skills"];

/// How many content rows fit in the panel before it scrolls with the cursor.
const VISIBLE_ROWS: usize = 12;

/// The content rows of the active `tab` (no header, no cursor markers), built
/// fresh from the live party, inventory, and database.
pub(super) fn tab_rows(
    tab: usize,
    data: &GameData,
    party: &Party,
    inventory: &Inventory,
) -> Vec<String> {
    match tab {
        0 => party_rows(data, party),
        1 => item_rows(data, inventory),
        _ => skill_rows(data),
    }
}

fn party_rows(data: &GameData, party: &Party) -> Vec<String> {
    party
        .snapshot()
        .iter()
        .map(|&id| match data.actor(id) {
            Some(a) => {
                format!(
                    "{} — {} — Lv{}   HP {}   SP {}",
                    i18n::tr(&a.name),
                    i18n::tr(&a.title),
                    a.level,
                    a.hp,
                    a.sp
                )
            }
            None => format!("#{id} (unknown)"),
        })
        .collect()
}

fn item_rows(data: &GameData, inventory: &Inventory) -> Vec<String> {
    let mut rows: Vec<String> = data
        .items
        .iter()
        .filter(|i| inventory.count(i.id) > 0)
        .map(|i| format!("{} ×{}", i18n::tr(&i.name), inventory.count(i.id)))
        .collect();
    if rows.is_empty() {
        rows.push("(no items)".to_string());
    }
    rows.push(String::new());
    rows.push(format!("Gold: {}", inventory.gold()));
    rows
}

fn skill_rows(data: &GameData) -> Vec<String> {
    data.skills
        .iter()
        .map(|s| format!("{}   (SP {})", i18n::tr(&s.name), s.sp_cost))
        .collect()
}

/// Render the tab header plus the visible window of `rows` around `cursor`,
/// marking the cursor row.
pub(super) fn compose(tab: usize, cursor: usize, rows: &[String]) -> String {
    let header = TABS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if i == tab {
                format!("[{name}]")
            } else {
                format!(" {name} ")
            }
        })
        .collect::<Vec<_>>()
        .join("   ");
    let start = viewport_start(cursor, rows.len());
    let mut out = format!("{header}          [S] Mentés   [Enter] választ\n\n");
    for (i, row) in rows.iter().enumerate().skip(start).take(VISIBLE_ROWS) {
        out.push_str(if i == cursor { "▶ " } else { "  " });
        out.push_str(row);
        out.push('\n');
    }
    out
}

/// The first row index of the scroll window that keeps `cursor` visible within
/// [`VISIBLE_ROWS`], never scrolling past the end.
fn viewport_start(cursor: usize, len: usize) -> usize {
    if len <= VISIBLE_ROWS {
        0
    } else {
        cursor
            .saturating_sub(VISIBLE_ROWS - 1)
            .min(len - VISIBLE_ROWS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit::{ITEM_HERB, data};

    #[test]
    fn party_row_shows_name_title_level_and_stats() {
        let rows = party_rows(&data(), &Party::default());
        assert_eq!(
            rows,
            vec!["Ron — Zsoldos — Lv2   HP 63   SP 37".to_string()]
        );
    }

    #[test]
    fn item_rows_list_only_held_items_and_always_end_with_gold() {
        let mut inv = Inventory::default();
        inv.add_gold(250);
        let empty = item_rows(&data(), &inv);
        assert_eq!(empty.first().unwrap(), "(no items)");
        assert_eq!(empty.last().unwrap(), "Gold: 250");
        inv.add_item(ITEM_HERB, 3);
        let held = item_rows(&data(), &inv);
        assert_eq!(held.first().unwrap(), "Gyógyfű ×3");
    }

    #[test]
    fn viewport_scrolls_to_keep_cursor_visible() {
        assert_eq!(viewport_start(0, 5), 0);
        assert_eq!(viewport_start(3, 40), 0);
        assert_eq!(viewport_start(VISIBLE_ROWS, 40), 1);
        assert_eq!(viewport_start(39, 40), 40 - VISIBLE_ROWS);
    }
}
