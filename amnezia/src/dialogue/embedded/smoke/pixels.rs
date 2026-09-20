use super::{LABELS, Probe};
use crate::font::bitmap::{BitmapFont, DEFAULT, PixelText, Run};
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
    let capture = LABELS.iter().position(|candidate| *candidate == label)?;
    let case = capture.saturating_sub(1) / 2;
    let partial = capture == 0;
    let phase = if capture % 2 == 0 { 96 } else { 64 };
    let lines = match case {
        0 if partial => vec!["Ron", "(S"],
        0 => vec!["Ron", "(Segítek neki!)", "(...)"],
        1 => vec!["Daren", "\"Hány csipet Terra-só legyen?\""],
        _ => vec!["Első sor", "Második sor", "Harmadik sor"],
    };
    let left = if case < 2 { 72 } else { 0 };
    let mut runs = lines
        .into_iter()
        .enumerate()
        .map(|(row, text)| {
            Run::new(
                text,
                left + i32::from(case == 0 && row > 0) * 12,
                2 + row as i32 * 16,
                DEFAULT,
            )
        })
        .collect::<Vec<_>>();
    if case > 0 {
        runs.extend((0..if case == 1 { 1 } else { 4 }).map(|digit| {
            Run::new(
                "0",
                left + 12 + digit * 12,
                if case == 1 { 34 } else { 50 },
                DEFAULT,
            )
        }));
    }
    let selection = if partial {
        None
    } else {
        Some(match case {
            0 => (82, 24, 228, phase),
            1 => (88, 40, 14, phase),
            _ => (52, 56, 14, phase),
        })
    };
    let server = world.resource::<AssetServer>();
    let skin = server.load::<Image>("graphics/System/System.png");
    let face = (case < 2).then(|| {
        server.load::<Image>(crate::assets::resolve_png(
            "FaceSet",
            if case == 0 { "Ron" } else { "Daren" },
        ))
    });
    let images = world.resource::<Assets<Image>>();
    let skin = images.get(&skin).unwrap();
    let glyphs = world.resource::<BitmapFont>().render(
        &PixelText {
            size: UVec2::new(304, 64),
            runs,
        },
        skin,
    );
    Some(Snapshot {
        pixels: crate::dialogue::view::prompts::smoke::pixels::reference(
            skin,
            &glyphs,
            face.map(|face| (images.get(&face).unwrap(), if case == 0 { 6 } else { 0 })),
            [160, 0, 80][case],
            case == 2,
            selection,
        ),
        checks: world.resource::<Probe>().pixels.clone(),
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
                "embedded prompt ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!("embedded prompt: 76800 body, choice/number, portrait and cursor pixels verified");
    }
}
