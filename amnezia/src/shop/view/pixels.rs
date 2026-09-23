use super::*;
use crate::font::bitmap::Run;
use crate::shop::{Mode, smoke::layout::Checks};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(in crate::shop) mod canvas;
mod party;
mod text;
use canvas::Canvas;

pub(crate) struct Snapshot {
    pixels: Vec<[u8; 4]>,
    checks: Arc<AtomicUsize>,
    label: String,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !selected(label) {
        return None;
    }
    let Screen::Shop(state) = world.resource::<Screen>() else {
        panic!("shop closed at {label}")
    };
    verify_state(state, label);
    let server = world.resource::<AssetServer>();
    let images = world.resource::<Assets<Image>>();
    let skin = server.load::<Image>("graphics/System/System.png");
    let skin = images.get(&skin).unwrap();
    let font = world.resource::<BitmapFont>();
    let face = world.resource::<crate::dialogue::Dialogue>().face.graphic();
    let mut canvas = Canvas::new();
    canvas.window(skin, (0, 0, 320, 32));
    canvas.window(skin, (0, 160, 320, 80));
    if matches!(state.phase, Phase::Command { .. } | Phase::Sell { .. }) {
        canvas.window(skin, (0, 32, 320, 128));
    } else {
        for rect in [
            (0, 32, 184, 128),
            (184, 32, 136, 48),
            (184, 80, 136, 48),
            (184, 128, 136, 32),
        ] {
            canvas.window(skin, rect);
        }
    }
    let cursor = match state.phase {
        Phase::Command { cursor, .. } => Some((
            12 + if face.is_some() { 72 } else { 0 },
            184 + cursor as u32 * 16,
            if face.is_some() { 224 } else { 296 },
            state.scene.command_frame,
        )),
        Phase::Buy { .. } if state.scene.updated => Some((
            4,
            40 + state.scene.buy.cursor_y as u32,
            176,
            state.scene.buy.cursor_frame,
        )),
        Phase::Sell { .. } => Some((
            4 + state.scene.sell.cursor_index as u32 % 2 * 160,
            40 + state.scene.sell.cursor_y as u32,
            152,
            state.scene.sell.cursor_frame,
        )),
        Phase::Number(_) => Some((154, 72, 20, state.scene.number_frame)),
        _ => None,
    };
    if let Some((left, top, width, phase)) = cursor {
        canvas.cursor(
            skin,
            (left, top, width, 16),
            if phase <= 10 { 64 } else { 96 },
        );
    }
    for (rect, content, offset) in text::contents(world, state, face.is_some()) {
        canvas.text(font, skin, rect, content, offset);
    }
    for (active, x, arrows) in [
        (
            matches!(state.phase, Phase::Buy { .. }),
            84,
            state.scene.buy.arrows,
        ),
        (
            matches!(state.phase, Phase::Sell { .. }),
            152,
            state.scene.sell.arrows,
        ),
    ] {
        if active {
            for (show, top, sy) in [(arrows[0], 32, 8), (arrows[1], 152, 16)] {
                if show {
                    canvas.blit(skin, (x, top), (40, sy, 16, 8), false);
                }
            }
        }
    }
    if let Some((name, index)) = face {
        let image = server.load::<Image>(crate::assets::resolve_png("FaceSet", name));
        canvas.blit(
            images.get(&image).unwrap(),
            (16, 176),
            (index % 4 * 48, index / 4 * 48, 48, 48),
            false,
        );
    }
    if !matches!(state.phase, Phase::Command { .. } | Phase::Sell { .. }) {
        party::draw(
            &mut canvas,
            world,
            skin,
            state.scene.item_id,
            state.scene.party_frame,
        );
    }
    if matches!(
        label,
        "shop-scene-fade-in" | "shop-scene-fade-out" | "shop-buy-fade-in"
    ) {
        assert_eq!(world.resource::<crate::transitions::Transition>().age(), 1);
        let factor = if label == "shop-scene-fade-out" {
            128
        } else {
            127
        };
        for color in &mut canvas.0 {
            for channel in &mut color[..3] {
                *channel = ((u32::from(*channel) * factor + 127) / 255) as u8;
            }
        }
    }
    Some(Snapshot {
        pixels: canvas.0,
        checks: world.resource::<Checks>().0.clone(),
        label: label.into(),
    })
}

fn selected(label: &str) -> bool {
    matches!(
        label,
        "shop-command"
            | "shop-scene-fade-in"
            | "shop-scene-fade-out"
            | "shop-buy-fade-in"
            | "shop-buy-quantity"
            | "shop-bought"
            | "shop-buy-return"
            | "shop-disabled-sale"
            | "shop-sell-quantity"
            | "shop-sold"
            | "shop-portrait-command"
            | "shop-equipped"
            | "shop-page"
            | "shop-scroll-first"
            | "shop-scroll-half"
            | "shop-scroll-done"
            | "shop-scroll-up-first"
            | "shop-scroll-up-half"
            | "shop-scroll-up-done"
            | "shop-sell-columns"
            | "shop-sell-scroll-first"
            | "shop-sell-scroll-half"
            | "shop-sell-scroll-done"
            | "shop-portrait-regreeting"
            | "shop-disabled-buy"
            | "shop-empty-sell"
            | "shop-third-style"
            | "shop-empty-buy"
            | "shop-resized"
    )
}

fn verify_state(state: &ShopState, label: &str) {
    if label == "shop-buy-fade-in" {
        assert!(matches!(state.phase, Phase::Buy { cursor: 0 }));
        assert!(!state.scene.updated);
        assert_eq!((state.scene.help_id, state.scene.item_id), (0, 0));
        return;
    }
    let expected = match label {
        "shop-command" | "shop-portrait-command" | "shop-third-style" | "shop-scene-fade-in" => {
            assert!(matches!(
                state.phase,
                Phase::Command {
                    cursor: 0,
                    regreet: false
                }
            ));
            return;
        }
        "shop-portrait-regreeting" | "shop-scene-fade-out" => {
            assert!(matches!(
                state.phase,
                Phase::Command {
                    cursor: 1,
                    regreet: true
                }
            ));
            return;
        }
        "shop-equipped" => (false, 0, 0, 0, 0, 1),
        "shop-page" => (false, 7, 16, 7, 96, 8),
        "shop-scroll-first" => (false, 8, 20, 7, 96, 8),
        "shop-scroll-half" => (false, 8, 24, 7, 96, 8),
        "shop-scroll-done" => (false, 8, 32, 8, 96, 9),
        "shop-scroll-up-first" => (false, 0, 12, 1, 0, 2),
        "shop-scroll-up-half" => (false, 0, 8, 1, 0, 2),
        "shop-scroll-up-done" => (false, 0, 0, 0, 0, 1),
        "shop-sell-columns" => (true, 0, 0, 0, 0, 1),
        "shop-sell-scroll-first" => (true, 15, 4, 13, 96, 14),
        "shop-sell-scroll-half" => (true, 15, 8, 13, 96, 14),
        "shop-sell-scroll-done" => (true, 15, 16, 15, 96, 16),
        "shop-disabled-sale" => (true, 0, 0, 0, 0, 1),
        "shop-buy-return" => (false, 1, 0, 1, 16, 7),
        "shop-disabled-buy" => (false, 0, 0, 0, 0, 7),
        "shop-empty-sell" => (true, 0, 0, 0, 0, 0),
        "shop-empty-buy" | "shop-resized" => (false, 0, 0, 0, 0, 0),
        "shop-buy-quantity" | "shop-sell-quantity" => {
            let Phase::Number(number) = &state.phase else {
                panic!("quantity at {label}")
            };
            assert_eq!(number.item_id, 7);
            assert_eq!(
                (number.mode, number.count),
                if label == "shop-buy-quantity" {
                    (Mode::Buy, 11)
                } else {
                    (Mode::Sell, 15)
                }
            );
            assert_eq!(state.scene.help_id, 0);
            return;
        }
        "shop-bought" | "shop-sold" => {
            assert!(if label == "shop-bought" {
                matches!(state.phase, Phase::Bought { item_id: 7, .. })
            } else {
                matches!(state.phase, Phase::Sold { item_id: 7, .. })
            });
            assert_eq!(state.scene.item_id, 7);
            assert_eq!(state.scene.help_id, 0);
            return;
        }
        _ => unreachable!(),
    };
    let (sell, index, offset, cursor, y, help) = expected;
    let actual = if sell {
        assert!(matches!(state.phase, Phase::Sell { cursor } if cursor == index));
        let list = &state.scene.sell;
        (
            list.index,
            list.offset,
            list.cursor_index,
            list.cursor_y,
            state.scene.help_id,
        )
    } else {
        assert!(matches!(state.phase, Phase::Buy { cursor } if cursor == index));
        let list = &state.scene.buy;
        (
            list.index,
            list.offset,
            list.cursor_index,
            list.cursor_y,
            state.scene.help_id,
        )
    };
    assert_eq!(actual, (index, offset, cursor, y, help), "{label}");
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
            "shop layout: 76800 reference window, text, portrait, party and cursor pixels verified ({})",
            self.label
        );
    }
}
