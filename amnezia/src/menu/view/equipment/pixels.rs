use crate::font::bitmap::BitmapFont;
use crate::menu::{
    MenuOpen, MenuScreen, MenuState,
    equip::{Scene, layout_smoke::Checks},
};
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod fixture;
use fixture::Fixture;

pub(crate) struct Snapshot {
    pixels: Vec<[u8; 4]>,
    checks: Arc<AtomicUsize>,
    label: String,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let expected = Fixture::from_label(label)?;
    assert!(world.resource::<MenuOpen>().0);
    assert_eq!(
        world.resource::<MenuState>().screen,
        MenuScreen::Equip {
            member: expected.member,
            slot: expected.slot,
            picking: expected.picking
        }
    );
    let scene = world.resource::<Scene>();
    assert_eq!(
        (scene.current, scene.preview, scene.help_id),
        (expected.current, expected.preview, expected.help),
        "{label}"
    );
    let nav = &scene.lists[expected.slot];
    assert_eq!(nav.offset, expected.offset as i32, "{label}");
    if let Some(cursor) = expected.picking {
        assert_eq!(
            (nav.index, nav.cursor_index, nav.cursor_y),
            (cursor, expected.cursor, expected.cursor_y as i32),
            "{label}"
        );
    }
    let handle = world
        .resource::<AssetServer>()
        .load("graphics/System/System.png");
    let skin = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let font = world.resource::<BitmapFont>();
    let texts = expected.text(world).map(|text| font.render(&text, skin));
    let mut pixels = Vec::with_capacity(320 * 240);
    for y in 0..240 {
        for x in 0..320 {
            let (wx, wy, width, height) = if y < 32 {
                (0, 0, 320, 32)
            } else if y < 128 && x < 124 {
                (0, 32, 124, 96)
            } else if y < 128 {
                (124, 32, 196, 96)
            } else {
                (0, 128, 320, 112)
            };
            let (lx, ly) = (x - wx, y - wy);
            let mut pixel = rgba(
                skin,
                background_coordinate(lx, width),
                background_coordinate(ly, height),
            );
            if lx < 8 || lx >= width - 8 || ly < 8 || ly >= height - 8 {
                pixel = over(
                    rgba(skin, 32 + coordinate(lx, width), coordinate(ly, height)),
                    pixel,
                );
            }
            for (visible, cx, cy, width, phase) in [
                (
                    true,
                    128,
                    40 + expected.slot as u32 * 16,
                    188,
                    scene.slot_frame,
                ),
                (
                    expected.picking.is_some(),
                    4 + expected.cursor as u32 % 2 * 160,
                    136 + expected.cursor_y,
                    152,
                    nav.cursor_frame,
                ),
            ] {
                if visible && (cx..cx + width).contains(&x) && (cy..cy + 16).contains(&y) {
                    pixel = over(
                        rgba(
                            skin,
                            if phase <= 10 { 64 } else { 96 } + coordinate(x - cx, width),
                            coordinate(y - cy, 16),
                        ),
                        pixel,
                    );
                }
            }
            for (index, tx, ty, width, height, offset) in [
                (0, 8, 8, 304, 16, 0),
                (1, 8, 40, 108, 80, 0),
                (2, 132, 40, 180, 80, 0),
                (3, 8, 136, 304, 96, expected.offset),
            ] {
                if (tx..tx + width).contains(&x) && (ty..ty + height).contains(&y) {
                    let sy = y - ty + offset;
                    if sy < texts[index].height() {
                        pixel = over(rgba(&texts[index], x - tx, sy), pixel);
                    }
                }
            }
            for (visible, top, sy) in [(nav.arrows[0], 128, 8), (nav.arrows[1], 232, 16)] {
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
        label: label.into(),
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
    let scale = (32 << 16) / length;
    ((2 * position + 1) * scale / 2 - 1) >> 16
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
                "{} ({x},{y}): expected {expected:?}, got {actual:?}",
                self.label
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "equipment menu: {} original window, text, preview, cursor and arrow pixels verified ({})",
            self.pixels.len(),
            self.label
        );
    }
}
