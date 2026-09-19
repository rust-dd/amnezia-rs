use crate::font::bitmap::{BitmapFont, PixelText, Run};
use crate::gamedata::GameData;
use crate::menu::{MenuOpen, MenuScreen, MenuState, items::smoke::Checks};
use crate::state::{Inventory, Party};
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod rows;

pub(crate) struct Snapshot {
    pixels: Vec<[u8; 4]>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let (skill, member, selected, value, cursor_y, cursor_height) = match label {
        "target-item" => (0, 0, 0, 2, 8, 48),
        "target-used" => (0, 0, 0, 1, 8, 48),
        "target-empty" => (0, 0, 0, 0, 8, 48),
        "target-four" => (0, 0, 3, 0, 182, 48),
        "target-party" => (9, 0, 3, 60, 8, 222),
        "target-self" => (49, 2, 0, 10, 124, 48),
        "target-skill" | "target-long-name" => (7, 0, 2, 8, 124, 48),
        "target-cast-ready" | "target-cast-first" | "target-cast-second" | "target-cast-full" => {
            (7, 1, 0, 15, 8, 48)
        }
        _ => return None,
    };
    assert!(world.resource::<MenuOpen>().0);
    assert_eq!(
        world.resource::<MenuState>().screen,
        if skill == 0 {
            MenuScreen::ItemTarget {
                item_id: 105,
                cursor: selected,
            }
        } else {
            MenuScreen::SkillTarget {
                member,
                skill_id: skill,
                cursor: selected,
            }
        }
    );
    if skill == 0 {
        assert_eq!(world.resource::<Inventory>().count(105), value);
    }
    let data = world.resource::<GameData>();
    let name = if skill == 0 {
        &data.item(105).unwrap().name
    } else {
        &data
            .skills
            .iter()
            .find(|definition| definition.id == skill)
            .unwrap()
            .name
    };
    let value = value.to_string();
    let server = world.resource::<AssetServer>();
    let images = world.resource::<Assets<Image>>();
    let handle = server.load("graphics/System/System.png");
    let skin = images.get(&handle).unwrap();
    let font = world.resource::<BitmapFont>();
    let name = font.render(
        &PixelText {
            size: UVec2::new(120, 16),
            runs: vec![Run::new(crate::i18n::tr(name), 0, 2, 0)],
        },
        skin,
    );
    let value = font.render(
        &PixelText {
            size: UVec2::new(120, 16),
            runs: vec![
                Run::new(if skill == 0 { "Tárgyak" } else { "Ár" }, 0, 2, 1),
                Run::new(&value, 120 - font.width(&value), 2, 0),
            ],
        },
        skin,
    );
    let text = font.render(&rows::text(world, label), skin);
    let mut pixels = vec![rgba(skin, 0, 32); 320 * 240];
    for (left, top, width, height) in [(0, 0, 136, 32), (0, 32, 136, 32), (136, 0, 184, 240)] {
        for y in 0..height {
            for x in 0..width {
                let mut pixel = rgba(
                    skin,
                    background_coordinate(x, width),
                    background_coordinate(y, height),
                );
                if x < 8 || x >= width - 8 || y < 8 || y >= height - 8 {
                    pixel = over(
                        rgba(skin, 32 + coordinate(x, width), coordinate(y, height)),
                        pixel,
                    );
                }
                pixels[((top + y) * 320 + left + x) as usize] = pixel;
            }
        }
    }
    let origin = if world.resource::<super::Clock>().phase <= 10 {
        64
    } else {
        96
    };
    for y in 0..cursor_height {
        for x in 0..120 {
            let pixel = &mut pixels[((cursor_y + y) * 320 + 196 + x) as usize];
            *pixel = over(
                rgba(
                    skin,
                    origin + coordinate(x, 120),
                    coordinate(y, cursor_height),
                ),
                *pixel,
            );
        }
    }
    for (index, id) in world.resource::<Party>().snapshot().iter().enumerate() {
        let actor = data.actor(*id).unwrap();
        let handle = server.load(crate::assets::resolve_png("FaceSet", &actor.face_name));
        let face = images.get(&handle).unwrap();
        for y in 0..48 {
            for x in 0..48 {
                let pixel = &mut pixels[(8 + index * 58 + y) * 320 + 144 + x];
                *pixel = over(
                    rgba(
                        face,
                        actor.face_index % 4 * 48 + x as u32,
                        actor.face_index / 4 * 48 + y as u32,
                    ),
                    *pixel,
                );
            }
        }
    }
    for (left, top, image) in [(8, 8, &name), (8, 40, &value), (144, 8, &text)] {
        for y in 0..image.height() {
            for x in 0..image.width() {
                let pixel = &mut pixels[((top + y) * 320 + left + x) as usize];
                *pixel = over(rgba(image, x, y), *pixel);
            }
        }
    }
    Some(Snapshot {
        pixels,
        checks: world.resource::<Checks>().target_pixels.clone(),
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
                "actor target ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "actor target: {} original window, portrait, text and cursor pixels verified",
            self.pixels.len()
        );
    }
}
