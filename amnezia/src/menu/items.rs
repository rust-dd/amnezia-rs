//! The Item command's held-item list: the field-usable recovery items the party
//! holds, with counts, the gold total beneath, and the scrolling viewport that
//! keeps the row cursor in sight. Selecting a field-usable item drills into the
//! member picker in [`super::use_item`]; this module only maps a row to an item id
//! and composes the list text.

use crate::gamedata::GameData;
use crate::i18n;
use crate::state::Inventory;

use super::use_item;

/// How many item rows fit before the list scrolls with the cursor.
const VISIBLE_ROWS: usize = 12;

/// The held item id under `cursor`, or `None` when the cursor sits past the last
/// held item (the blank spacer or the gold line, which select nothing). The order
/// matches [`use_item::held_item_ids`], so a cursor row maps back to its item.
pub(super) fn item_at(cursor: usize, data: &GameData, inventory: &Inventory) -> Option<u32> {
    use_item::held_item_ids(data, inventory)
        .get(cursor)
        .copied()
}

/// The selectable rows: one per held item. Zero when the bag is empty, so the
/// cursor stays put and a confirm selects nothing.
pub(super) fn selectable(data: &GameData, inventory: &Inventory) -> usize {
    use_item::held_item_ids(data, inventory).len()
}

/// The display rows: each held item with its count, then a blank spacer and the
/// gold line. An empty bag shows a single placeholder row.
fn rows(data: &GameData, inventory: &Inventory) -> Vec<String> {
    let mut rows: Vec<String> = data
        .items
        .iter()
        .filter(|i| inventory.count(i.id) > 0)
        .map(|i| format!("{} ×{}", i18n::tr(&i.name), inventory.count(i.id)))
        .collect();
    if rows.is_empty() {
        rows.push("(nincs tárgy)".to_string());
    }
    rows.push(String::new());
    rows.push(format!("Arany: {}", inventory.gold()));
    rows
}

/// Compose the item list and the composed-text line its windowskin cursor sits
/// on (`None` for an empty bag). The held items scroll within a viewport; the
/// line indexes into the returned text — header included — so [`super::view`] can
/// place the cursor rectangle over the selected row. The spacer and gold rows are
/// never highlighted (the cursor can't reach them).
pub(super) fn compose_list(
    cursor: usize,
    data: &GameData,
    inventory: &Inventory,
) -> (String, Option<usize>) {
    let rows = rows(data, inventory);
    let selectable = selectable(data, inventory);
    let start = viewport_start(cursor, rows.len(), VISIBLE_ROWS);
    let mut lines = vec![String::from("- Tárgyak -"), String::new()];
    let mut cursor_line = None;
    for (i, row) in rows.iter().enumerate().skip(start).take(VISIBLE_ROWS) {
        if i == cursor && i < selectable {
            cursor_line = Some(lines.len());
        }
        lines.push(row.clone());
    }
    lines.push(String::new());
    lines.push(String::from("[Esc] vissza"));
    (lines.join("\n"), cursor_line)
}

/// The first row of the scroll window that keeps `cursor` visible within
/// `visible` rows, never scrolling past the end. Shared with the skill list.
pub(super) fn viewport_start(cursor: usize, len: usize, visible: usize) -> usize {
    if len <= visible {
        0
    } else {
        cursor.saturating_sub(visible - 1).min(len - visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit::{ITEM_HERB, data};

    #[test]
    fn item_at_maps_a_row_to_its_held_item_and_nothing_past_the_end() {
        let mut inv = Inventory::default();
        inv.add_item(ITEM_HERB, 3);
        assert_eq!(item_at(0, &data(), &inv), Some(ITEM_HERB));
        // Past the single held item: the spacer/gold rows select nothing.
        assert_eq!(item_at(1, &data(), &inv), None);
    }

    #[test]
    fn list_shows_held_items_with_counts_a_gold_line_and_the_cursor_row() {
        let mut inv = Inventory::default();
        inv.add_item(ITEM_HERB, 3);
        inv.add_gold(250);
        let (text, cursor_line) = compose_list(0, &data(), &inv);
        assert!(text.contains("Gyógyfű ×3"), "held item: {text}");
        assert!(text.contains("Arany: 250"), "gold line: {text}");
        // Line 0 is the header, line 1 blank, so the first item is on line 2.
        assert_eq!(cursor_line, Some(2), "cursor over the first item: {text}");
    }

    #[test]
    fn empty_bag_shows_a_placeholder_and_selects_nothing() {
        let inv = Inventory::default();
        assert_eq!(selectable(&data(), &inv), 0);
        let (text, cursor_line) = compose_list(0, &data(), &inv);
        assert!(text.contains("(nincs tárgy)"), "placeholder: {text}");
        assert_eq!(cursor_line, None, "nothing to highlight: {text}");
    }

    #[test]
    fn viewport_scrolls_to_keep_cursor_visible() {
        assert_eq!(viewport_start(0, 5, VISIBLE_ROWS), 0);
        assert_eq!(viewport_start(3, 40, VISIBLE_ROWS), 0);
        assert_eq!(viewport_start(VISIBLE_ROWS, 40, VISIBLE_ROWS), 1);
        assert_eq!(viewport_start(39, 40, VISIBLE_ROWS), 40 - VISIBLE_ROWS);
    }
}
