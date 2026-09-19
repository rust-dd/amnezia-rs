use crate::font::bitmap::{BitmapFont, DEFAULT, DISABLED, PixelText, Run};
use crate::gamedata::GameData;
use crate::menu::{
    MenuOpen, MenuScreen, MenuState,
    items::{List, smoke::Checks},
};
use crate::state::Inventory;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) struct Snapshot {
    pixels: Vec<[u8; 4]>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let (selected, offset, cursor, cursor_y, description) = match label {
        "items-empty" | "items-early" | "items-cleared" | "items-up-done" => (0, 0, 0, 0, 0),
        "items-disabled" => (1, 0, 1, 0, 1),
        "items-scroll-first" => (24, 4, 22, 176, 22),
        "items-scroll-half" => (24, 8, 22, 176, 22),
        "items-scroll-done" | "items-returned" => (24, 16, 24, 176, 24),
        "items-depleted" => (23, 16, 23, 160, 23),
        "items-up-half" => (0, 8, 2, 0, 2),
        _ => return None,
    };
    assert!(world.resource::<MenuOpen>().0);
    assert_eq!(
        world.resource::<MenuState>().screen,
        MenuScreen::ItemList { cursor: selected }
    );
    let nav = &world.resource::<List>().navigation;
    assert_eq!(
        (
            nav.index,
            nav.offset,
            nav.cursor_index,
            nav.cursor_y,
            nav.help_index
        ),
        (selected, offset, cursor, cursor_y, description),
        "{label}"
    );
    let empty = matches!(label, "items-empty" | "items-cleared");
    let depleted = matches!(label, "items-depleted" | "items-up-half" | "items-up-done");
    let mut ids = if empty {
        Vec::new()
    } else {
        (1..=24).collect::<Vec<u32>>()
    };
    if !empty && !depleted {
        ids.push(105);
    }
    let data = world.resource::<GameData>();
    let inventory = world.resource::<Inventory>();
    let mut runs = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let item = data.item(*id).unwrap();
        let count = if *id == 105 {
            if label == "items-returned" { 1 } else { 2 }
        } else {
            *id * 3
        };
        assert_eq!(inventory.count(*id), count);
        let color = if *id == 105 { DEFAULT } else { DISABLED };
        let x = (index % 2 * 160) as i32;
        let y = (index / 2 * 16 + 2) as i32;
        runs.push(Run::new(&item.name, x, y, color));
        runs.push(Run::new(format!(":{count:>3}"), x + 120, y, color));
    }
    let description = ids
        .get(description)
        .map(|id| data.item(*id).unwrap().description.as_str())
        .unwrap_or("");
    let handle = world
        .resource::<AssetServer>()
        .load("graphics/System/System.png");
    let skin = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let font = world.resource::<BitmapFont>();
    let help = font.render(
        &PixelText {
            size: UVec2::new(304, 16),
            runs: vec![Run::new(description, 0, 2, DEFAULT)],
        },
        skin,
    );
    let entries = font.render(
        &PixelText {
            size: UVec2::new(304, 16 * ids.len().div_ceil(2).max(12) as u32),
            runs,
        },
        skin,
    );
    let mut pixels = Vec::with_capacity(320 * 240);
    for y in 0..240 {
        for x in 0..320 {
            let (local_y, height) = if y < 32 { (y, 32) } else { (y - 32, 208) };
            let mut pixel = rgba(
                skin,
                background_coordinate(x, 320),
                background_coordinate(local_y, height),
            );
            if !(8..312).contains(&x) || local_y < 8 || local_y >= height - 8 {
                pixel = over(
                    rgba(skin, 32 + coordinate(x, 320), coordinate(local_y, height)),
                    pixel,
                );
            }
            let cx = 4 + cursor as u32 % 2 * 160;
            let cy = 40 + cursor_y as u32;
            if (cx..cx + 152).contains(&x) && (cy..cy + 16).contains(&y) {
                let origin = if nav.cursor_frame <= 10 { 64 } else { 96 };
                pixel = over(
                    rgba(
                        skin,
                        origin + coordinate(x - cx, 152),
                        coordinate(y - cy, 16),
                    ),
                    pixel,
                );
            }
            if (8..312).contains(&x) {
                if (8..24).contains(&y) {
                    pixel = over(rgba(&help, x - 8, y - 8), pixel);
                } else if (40..232).contains(&y) {
                    let source_y = y - 40 + offset as u32;
                    if source_y < entries.height() {
                        pixel = over(rgba(&entries, x - 8, source_y), pixel);
                    }
                }
            }
            for (visible, top, sy) in [(nav.arrows[0], 32, 8), (nav.arrows[1], 232, 16)] {
                if visible && (152..168).contains(&x) && (top..top + 8).contains(&y) {
                    pixel = over(rgba(skin, 40 + x - 152, sy + y - top), pixel);
                }
            }
            pixels.push(pixel);
        }
    }
    Some(Snapshot {
        pixels,
        checks: world.resource::<Checks>().pixels.clone(),
    })
}

fn coordinate(position: u32, length: u32) -> u32 {
    if position < 8 {
        position
    } else if position >= length - 8 {
        24 + position - (length - 8)
    } else {
        8 + position % 16
    }
}

fn background_coordinate(position: u32, length: u32) -> u32 {
    let fixed_scale = (32 << 16) / length;
    ((2 * position + 1) * fixed_scale / 2 - 1) >> 16
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn over(foreground: [u8; 4], background: [u8; 4]) -> [u8; 4] {
    let alpha = u32::from(foreground[3]);
    let mut pixel = [0, 0, 0, 255];
    for i in 0..3 {
        pixel[i] = ((u32::from(foreground[i]) * alpha + u32::from(background[i]) * (255 - alpha))
            / 255) as u8;
    }
    pixel
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for (index, expected) in self.pixels.iter().enumerate() {
            let (x, y) = (index as u32 % 320, index as u32 / 320);
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.abs_diff(*b) <= 1),
                "item menu ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "item menu: {} original frame, description, list, cursor and arrow pixels verified",
            self.pixels.len()
        );
    }
}
