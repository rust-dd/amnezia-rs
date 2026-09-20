use crate::font::bitmap::{PixelText, Run};
use crate::gamedata::GameData;
use crate::menu::equip::layout_smoke::{ARMOR, LONG_NAME};
use crate::terms::Terms;
use bevy::prelude::*;

pub(super) struct Fixture {
    pub slot: usize,
    pub picking: Option<usize>,
    pub offset: u32,
    pub cursor: usize,
    pub cursor_y: u32,
    pub help: u32,
    pub current: [u32; 4],
    pub preview: Option<[u32; 4]>,
    before_swap: bool,
    fixed: bool,
    long: bool,
}

impl Fixture {
    pub(super) fn from_label(label: &str) -> Option<Self> {
        let (slot, picking, offset, cursor, cursor_y, help, preview) = match label {
            "equipment-slots" => (0, None, 0, 0, 0, 1, None),
            "equipment-preview" | "equipment-early" => {
                (0, Some(0), 0, 0, 0, 2, Some([45, 37, 20, 25]))
            }
            "equipment-preview-lag" => (0, Some(1), 0, 1, 0, 3, Some([45, 37, 20, 25])),
            "equipment-preview-changed" => (0, Some(1), 0, 1, 0, 3, Some([85, 37, 20, 25])),
            "equipment-equipped" | "equipment-long" => (0, None, 0, 0, 0, 2, None),
            "equipment-weaker" => (0, Some(0), 0, 0, 0, 1, Some([25, 32, 20, 25])),
            "equipment-armor" => (2, None, 0, 0, 0, 64, None),
            "equipment-scroll-first" => (2, Some(13), 4, 11, 80, 79, Some([45, 117, 20, 25])),
            "equipment-scroll-half" => (2, Some(13), 8, 11, 80, 79, Some([45, 117, 20, 25])),
            "equipment-scroll-done" => (2, Some(13), 16, 13, 80, 153, Some([45, 117, 20, 25])),
            "equipment-unequip" => (2, Some(18), 64, 18, 80, 0, Some([45, 32, 20, 25])),
            "equipment-inactive" => (2, None, 64, 18, 80, 64, None),
            "equipment-reopened" => (2, Some(0), 0, 0, 0, 64, Some([45, 37, 20, 25])),
            "equipment-accessory" | "equipment-wrap" => (4, None, 0, 0, 0, 0, None),
            "equipment-fixed" => (0, None, 0, 0, 0, 41, None),
            _ => return None,
        };
        let before_swap = matches!(
            label,
            "equipment-slots"
                | "equipment-preview"
                | "equipment-preview-lag"
                | "equipment-preview-changed"
                | "equipment-early"
        );
        let fixed = label == "equipment-fixed";
        let current = if fixed {
            [262, 312, 188, 240]
        } else if before_swap {
            [25, 32, 20, 25]
        } else {
            [45, 37, 20, 25]
        };
        Some(Self {
            slot,
            picking,
            offset,
            cursor,
            cursor_y,
            help,
            current,
            preview,
            before_swap,
            fixed,
            long: label == "equipment-long",
        })
    }

    pub(super) fn text(&self, world: &World) -> [PixelText; 4] {
        let data = world.resource::<GameData>();
        let terms = &world.resource::<Terms>().0;
        let name = if self.fixed {
            "Dianos"
        } else if self.long {
            LONG_NAME
        } else {
            "Áron"
        };
        let mut status = vec![Run::new(name, 0, 2, 0)];
        for (i, label) in [&terms.attack, &terms.defense, &terms.spirit, &terms.agility]
            .into_iter()
            .enumerate()
        {
            let y = 18 + i as i32 * 16;
            let old = self.current[i].to_string();
            status.push(Run::new(label, 0, y, 1));
            status.push(Run::new(&old, 78 - old.len() as i32 * 6, y, 0));
            if let Some(next) = self.preview {
                let new = next[i].to_string();
                let color = if next[i] > self.current[i] {
                    2
                } else if next[i] < self.current[i] {
                    3
                } else {
                    0
                };
                status.push(Run::new(">", 81, y, 1));
                status.push(Run::new(&new, 108 - new.len() as i32 * 6, y, color));
            }
        }
        let worn = if self.fixed {
            [41, 62, 81, 103, 0]
        } else {
            [if self.before_swap { 1 } else { 2 }, 0, 64, 83, 0]
        };
        let mut slots = Vec::new();
        for (i, label) in [
            &terms.weapon,
            &terms.shield,
            &terms.armor,
            &terms.helmet,
            &terms.accessory,
        ]
        .into_iter()
        .enumerate()
        {
            let y = 2 + i as i32 * 16;
            slots.push(Run::new(label, 0, y, 1));
            if let Some(item) = data.item(worn[i]) {
                slots.push(Run::new(&item.name, 60, y, 0));
            }
        }
        let ids = if self.fixed {
            Vec::new()
        } else if self.slot == 2 {
            ARMOR.into_iter().chain([0]).collect::<Vec<_>>()
        } else if self.slot == 4 {
            vec![0]
        } else if self.before_swap {
            vec![2, 3, 0]
        } else {
            vec![1, 2, 3, 0]
        };
        let mut entries = Vec::new();
        for (i, id) in ids.iter().enumerate() {
            if *id == 0 {
                continue;
            }
            let x = i as i32 % 2 * 160;
            let y = i as i32 / 2 * 16 + 2;
            entries.push(Run::new(&data.item(*id).unwrap().name, x, y, 0));
            entries.push(Run::new(
                if *id == 2 && self.before_swap {
                    ":  2"
                } else {
                    ":  1"
                },
                x + 120,
                y,
                0,
            ));
        }
        let description = data
            .item(self.help)
            .map(|item| item.description.clone())
            .unwrap_or_default();
        [
            PixelText {
                size: UVec2::new(304, 16),
                runs: vec![Run::new(description, 0, 2, 0)],
            },
            PixelText {
                size: UVec2::new(108, 80),
                runs: status,
            },
            PixelText {
                size: UVec2::new(180, 80),
                runs: slots,
            },
            PixelText {
                size: UVec2::new(304, ids.len().div_ceil(2).max(6) as u32 * 16),
                runs: entries,
            },
        ]
    }
}
