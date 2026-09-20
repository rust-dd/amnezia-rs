//! Inventory ordering and the original two-column field-item navigation.

mod list;
pub(crate) mod smoke;

pub(super) use list::{List, update};

use crate::gamedata::GameData;
use crate::state::Inventory;

use super::use_item;

/// The held item under `cursor`, using the same ordering as the item window.
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

/// The first row of the remaining legacy equipment-list viewport.
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
        assert_eq!(item_at(1, &data(), &inv), None);
    }

    #[test]
    fn viewport_scrolls_to_keep_cursor_visible() {
        assert_eq!(viewport_start(0, 5, 8), 0);
        assert_eq!(viewport_start(3, 40, 8), 0);
        assert_eq!(viewport_start(8, 40, 8), 1);
        assert_eq!(viewport_start(39, 40, 8), 32);
    }
}
