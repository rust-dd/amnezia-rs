use super::*;
use crate::font::bitmap::{BitmapFont, DEFAULT, DISABLED, PixelText, Run};
use crate::shop::view::pixels::canvas::Canvas;

pub(crate) struct Snapshot {
    pixels: Vec<[u8; 4]>,
    checked: Arc<AtomicUsize>,
    label: String,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let (index, _) = fixtures::LABELS
        .iter()
        .enumerate()
        .find_map(|(index, labels)| {
            labels
                .iter()
                .position(|value| *value == label)
                .map(|phase| (index, phase))
        })
        .or_else(|| (label == "inn-asleep").then_some((4, 0)))?;
    let mut canvas = Canvas::new();
    if label != "inn-asleep" {
        let case = fixtures::case(index);
        let server = world.resource::<AssetServer>();
        let images = world.resource::<Assets<Image>>();
        let skin = server.load::<Image>("graphics/System/System.png");
        let skin = images.get(&skin).unwrap();
        let font = world.resource::<BitmapFont>();
        if !case.transparent {
            canvas.window(skin, (0, case.top, 320, 80));
        }
        let left = if case.face { 72 } else { 0 };
        let phase = world
            .resource::<crate::dialogue::PromptClock>()
            .source_x(0, false) as u32;
        canvas.cursor(skin, (10 + left, case.top + 40, 300 - left, 16), phase);
        let lines = if index == 5 {
            vec![
                "Mindössze  30GP  egy éjszaka!".into(),
                "Itt alszik?".into(),
                "Igen".into(),
                "Nem".into(),
            ]
        } else {
            vec![
                format!("Egy éjszaka  {}GP !", case.cost),
                "Bejegyezhetem mára?".into(),
                "Igen".into(),
                "Nem".into(),
            ]
        };
        canvas.text(
            font,
            skin,
            (8, case.top + 8, 304, 64),
            PixelText {
                size: UVec2::new(304, 64),
                runs: lines
                    .into_iter()
                    .enumerate()
                    .map(|(row, text)| {
                        Run::new(
                            text,
                            left as i32 + if row >= 2 { 12 } else { 0 },
                            2 + row as i32 * 16,
                            if index == 0 && row == 2 {
                                DISABLED
                            } else {
                                DEFAULT
                            },
                        )
                    })
                    .collect(),
            },
            0,
        );
        if case.face {
            let face = server.load::<Image>(crate::assets::resolve_png("FaceSet", "Ron"));
            canvas.blit(
                images.get(&face).unwrap(),
                (16, case.top + 16),
                (96, 48, 48, 48),
                false,
            );
        }
        let gold_top = if case.top == 0 { 208 } else { 0 };
        canvas.window(skin, (232, gold_top, 88, 32));
        let gold = case.gold.to_string();
        canvas.text(
            font,
            skin,
            (240, gold_top + 8, 72, 16),
            PixelText {
                size: UVec2::new(72, 16),
                runs: vec![
                    Run::new("GP", 60, 2, 1),
                    Run::new(&gold, 60 - gold.len() as i32 * 6, 2, DEFAULT),
                ],
            },
            0,
        );
    }
    Some(Snapshot {
        pixels: canvas.0,
        checked: world.resource::<Probe>().pixels.clone(),
        label: label.into(),
    })
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
                "{} ({x}, {y}): expected {expected:?}, got {actual:?}",
                self.label
            );
        }
        self.checked.fetch_add(1, Ordering::Relaxed);
        info!(
            "inn: 76800 reference message, gold, portrait and cursor pixels verified ({})",
            self.label
        );
    }
}
