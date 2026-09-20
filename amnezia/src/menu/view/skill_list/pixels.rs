use crate::font::bitmap::BitmapFont;
use crate::menu::{
    MenuOpen, MenuScreen, MenuState,
    skills::{List, smoke::Checks},
};
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod contents;

pub(crate) struct Snapshot {
    pixels: Vec<[u8; 4]>,
    checks: Arc<AtomicUsize>,
}

struct Fixture {
    member: usize,
    selected: usize,
    offset: u32,
    cursor: usize,
    cursor_y: u32,
    help: usize,
    sp: i32,
    half: bool,
    poison: bool,
    empty: bool,
    long: bool,
}

impl Fixture {
    fn from_label(label: &str) -> Option<Self> {
        let (member, selected, offset, cursor, cursor_y, help, sp) = match label {
            "skills-early" => (0, 0, 0, 0, 0, 0, 5),
            "skills-tiffany" => (1, 0, 0, 0, 0, 0, 75),
            "skills-used" => (1, 0, 0, 0, 0, 0, 60),
            "skills-used-again" => (1, 0, 0, 0, 0, 0, 45),
            "skills-grid" => (0, 0, 0, 0, 0, 0, 5),
            "skills-scroll-first" => (0, 21, 4, 19, 144, 19, 5),
            "skills-scroll-half" => (0, 21, 8, 19, 144, 19, 5),
            "skills-scroll-done" => (0, 21, 16, 21, 144, 21, 5),
            "skills-bottom" => (0, 24, 48, 24, 144, 24, 5),
            "skills-half-cost" | "skills-returned" | "skills-long-name" => {
                (0, 24, 48, 24, 144, 24, 8)
            }
            "skills-up-first" => (0, 4, 44, 6, 0, 6, 8),
            "skills-up-half" => (0, 4, 40, 6, 0, 6, 8),
            "skills-up-done" => (0, 4, 32, 4, 0, 4, 8),
            "skills-no-sp" => (0, 4, 32, 4, 0, 4, 0),
            "skills-cleared" => (0, 0, 0, 0, 0, 0, 8),
            _ => return None,
        };
        let half = matches!(
            label,
            "skills-half-cost"
                | "skills-returned"
                | "skills-up-first"
                | "skills-up-half"
                | "skills-up-done"
                | "skills-no-sp"
                | "skills-long-name"
                | "skills-cleared"
        );
        Some(Self {
            member,
            selected,
            offset,
            cursor,
            cursor_y,
            help,
            sp,
            half,
            poison: half,
            empty: matches!(label, "skills-early" | "skills-cleared"),
            long: matches!(label, "skills-long-name" | "skills-cleared"),
        })
    }
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let expected = Fixture::from_label(label)?;
    assert!(world.resource::<MenuOpen>().0);
    assert_eq!(
        world.resource::<MenuState>().screen,
        MenuScreen::SkillList {
            member: expected.member,
            cursor: expected.selected
        }
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
        (
            expected.selected,
            expected.offset as i32,
            expected.cursor,
            expected.cursor_y as i32,
            expected.help
        ),
        "{label}"
    );
    let handle = world
        .resource::<AssetServer>()
        .load("graphics/System/System.png");
    let skin = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let font = world.resource::<BitmapFont>();
    let [help, status, entries] =
        contents::text(world, &expected).map(|text| font.render(&text, skin));
    let mut pixels = Vec::with_capacity(320 * 240);
    for y in 0..240 {
        for x in 0..320 {
            let (local_y, height) = if y < 32 {
                (y, 32)
            } else if y < 64 {
                (y - 32, 32)
            } else {
                (y - 64, 176)
            };
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
            let cx = 4 + expected.cursor as u32 % 2 * 160;
            let cy = 72 + expected.cursor_y;
            if (cx..cx + 152).contains(&x) && (cy..cy + 16).contains(&y) {
                let phase = if nav.cursor_frame <= 10 { 64 } else { 96 };
                pixel = over(
                    rgba(
                        skin,
                        phase + coordinate(x - cx, 152),
                        coordinate(y - cy, 16),
                    ),
                    pixel,
                );
            }
            if (8..312).contains(&x) {
                if (8..24).contains(&y) {
                    pixel = over(rgba(&help, x - 8, y - 8), pixel);
                } else if (40..56).contains(&y) {
                    pixel = over(rgba(&status, x - 8, y - 40), pixel);
                } else if (72..232).contains(&y) {
                    let sy = y - 72 + expected.offset;
                    if sy < entries.height() {
                        pixel = over(rgba(&entries, x - 8, sy), pixel);
                    }
                }
            }
            for (visible, top, sy) in [(nav.arrows[0], 64, 8), (nav.arrows[1], 232, 16)] {
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
                "skill menu ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "skill menu: {} original window, help, status, list, cursor and arrow pixels verified",
            self.pixels.len()
        );
    }
}
