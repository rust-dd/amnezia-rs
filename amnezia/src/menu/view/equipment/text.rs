use super::*;
use crate::font::bitmap::{DEFAULT, Run};
use crate::i18n::tr;

pub(super) fn help(id: u32, data: &GameData) -> PixelText {
    PixelText {
        size: UVec2::new(304, 16),
        runs: vec![Run::new(
            data.item(id)
                .map(|item| tr(&item.description))
                .unwrap_or_default(),
            0,
            2,
            DEFAULT,
        )],
    }
}

pub(super) fn slots(
    actor: &amnezia_data::ActorDef,
    data: &GameData,
    equipment: &Equipment,
    terms: &Terms,
) -> PixelText {
    let labels = [
        &terms.0.weapon,
        if actor.two_weapons {
            &terms.0.weapon
        } else {
            &terms.0.shield
        },
        &terms.0.armor,
        &terms.0.helmet,
        &terms.0.accessory,
    ];
    let mut runs = Vec::new();
    for (index, (label, id)) in labels.into_iter().zip(equipment.slots(actor)).enumerate() {
        let y = index as i32 * 16 + 2;
        runs.push(Run::new(tr(label), 0, y, 1));
        if let Some(item) = data.item(id) {
            runs.push(Run::new(tr(&item.name), 60, y, DEFAULT));
        }
    }
    PixelText {
        size: UVec2::new(180, 80),
        runs,
    }
}

pub(super) fn entries(ids: &[u32], data: &GameData, inventory: &Inventory) -> PixelText {
    let mut runs = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let x = index as i32 % 2 * 160;
        let y = index as i32 / 2 * 16 + 2;
        runs.push(Run::clear(x, y, 144, 12));
        let Some(item) = data.item(*id) else { continue };
        runs.push(Run::new(tr(&item.name), x, y, DEFAULT));
        runs.push(Run::new(
            format!(":{:>3}", inventory.count(*id)),
            x + 120,
            y,
            DEFAULT,
        ));
    }
    PixelText {
        size: UVec2::new(304, ids.len().div_ceil(2).max(6) as u32 * 16),
        runs,
    }
}

pub(in crate::menu) fn status(
    name: &str,
    current: [u32; 4],
    preview: Option<[u32; 4]>,
    terms: &Terms,
    font: &BitmapFont,
) -> PixelText {
    let mut runs = vec![Run::new(tr(name), 0, 2, DEFAULT)];
    for (index, label) in [
        &terms.0.attack,
        &terms.0.defense,
        &terms.0.spirit,
        &terms.0.agility,
    ]
    .into_iter()
    .enumerate()
    {
        let y = 18 + index as i32 * 16;
        let old = current[index].to_string();
        runs.push(Run::new(tr(label), 0, y, 1));
        runs.push(Run::new(&old, 78 - font.width(&old), y, DEFAULT));
        if let Some(preview) = preview {
            let value = preview[index];
            let color = match value.cmp(&current[index]) {
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 2,
                std::cmp::Ordering::Less => 3,
            };
            let new = value.to_string();
            runs.push(Run::new(">", 81, y, 1));
            runs.push(Run::new(&new, 108 - font.width(&new), y, color));
        }
    }
    PixelText {
        size: UVec2::new(108, 80),
        runs,
    }
}
