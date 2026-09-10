use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

const BACKGROUND: [u8; 3] = [18, 48, 80];

#[derive(Resource)]
struct Probe {
    verified: Arc<AtomicU32>,
}

fn sample() -> PixelText {
    PixelText {
        size: UVec2::new(320, 240),
        runs: (0..20)
            .map(|color| {
                Run::new(
                    "ŐűAa09",
                    10 + (color as i32 % 5) * 60,
                    20 + (color as i32 / 5) * 50,
                    color,
                )
            })
            .collect(),
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        world.insert_resource(Probe {
            verified: Arc::new(AtomicU32::new(0)),
        });
        world
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(BACKGROUND[0], BACKGROUND[1], BACKGROUND[2])),
                GlobalZIndex(4000),
            ))
            .with_children(|root| {
                root.spawn((
                    Node {
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    sample(),
                ));
            });
    }
    if frame == 600 {
        world.insert_resource(BitmapFont::from_id(1));
    }
    match frame {
        320 => Some("font-palette-rm2000"),
        640 => Some("font-palette-rmg2000"),
        _ => None,
    }
}

pub(crate) struct Snapshot {
    expected: Image,
    verified: Arc<AtomicU32>,
}

pub(crate) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    if !label.starts_with("font-palette-") {
        return None;
    }
    let palette = world.resource::<Palette>();
    let system = world
        .resource::<Assets<Image>>()
        .get(&palette.source)
        .unwrap();
    Some(Snapshot {
        expected: world.resource::<BitmapFont>().render(&sample(), system),
        verified: world.resource::<Probe>().verified.clone(),
    })
}

impl Snapshot {
    pub fn verify(&self, actual: &Image) {
        for y in 0..240 {
            for x in 0..320 {
                let pixel = self
                    .expected
                    .get_color_at(x, y)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                let expected = if pixel[3] == 0 {
                    BACKGROUND
                } else {
                    [pixel[0], pixel[1], pixel[2]]
                };
                let got = actual
                    .get_color_at(x * actual.width() / 320, y * actual.height() / 240)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                for channel in 0..3 {
                    assert!(
                        got[channel].abs_diff(expected[channel]) <= 1,
                        "font palette ({x},{y}): expected {expected:?}, got {got:?}"
                    );
                }
            }
        }
        self.verified.fetch_add(1, Ordering::SeqCst);
        info!("bitmap font palette: 76800 GPU pixels verified");
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Probe>().verified.load(Ordering::SeqCst), 2);
}
