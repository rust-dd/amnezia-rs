use super::*;
use crate::dialogue::{MessageOptions, MessagePosition, MessageTransparent};
use crate::font::bitmap::{BitmapFont, PixelText};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(in crate::dialogue) mod pixels;

const LABELS: [&str; 12] = [
    "prompt-choice-bottom-a",
    "prompt-choice-bottom-b",
    "prompt-choice-portrait-a",
    "prompt-choice-portrait-b",
    "prompt-choice-transparent-a",
    "prompt-choice-transparent-b",
    "prompt-number-bottom-a",
    "prompt-number-bottom-b",
    "prompt-number-transparent-a",
    "prompt-number-transparent-b",
    "prompt-number-portrait-a",
    "prompt-number-portrait-b",
];

struct Case {
    top: u32,
    face: bool,
    transparent: bool,
    digits: u32,
}

fn case(index: usize) -> Case {
    Case {
        top: [160, 0, 80, 160, 0, 80][index],
        face: matches!(index, 1 | 2 | 4 | 5),
        transparent: matches!(index, 2 | 4),
        digits: match index {
            3 | 5 => 4,
            4 => 1,
            _ => 0,
        },
    }
}

fn labels(index: usize) -> Vec<String> {
    if index == 2 {
        vec![
            "Árvíztűrő tükörfúrógép.".into(),
            "W".repeat(70),
            "Tovább".into(),
        ]
    } else {
        ["Első", "Második", "Harmadik", "Negyedik"]
            .map(str::to_string)
            .to_vec()
    }
}

#[derive(Resource)]
struct Probe {
    black: Entity,
    captured: u16,
    pixels: Arc<AtomicUsize>,
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1450 {
        let black = world
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                GlobalZIndex(99),
            ))
            .id();
        world.insert_resource(Probe {
            black,
            captured: 0,
            pixels: Arc::default(),
        });
    }
    if frame == 1820 {
        world.resource_mut::<Choice>().active = false;
        world.resource_mut::<InputNumber>().active = false;
        let black = world.resource::<Probe>().black;
        world.despawn(black);
    }
    if !(1450..1810).contains(&frame) {
        return None;
    }
    let index = ((frame - 1450) / 60) as usize;
    let age = (frame - 1450) % 60;
    let case = case(index);
    if age == 0 {
        assert!(!world.resource::<Dialogue>().active);
        world.resource_mut::<Choice>().active = false;
        world.resource_mut::<InputNumber>().active = false;
        world.insert_resource(match case.top {
            0 => MessagePosition::Top,
            80 => MessagePosition::Middle,
            _ => MessagePosition::Bottom,
        });
        world.insert_resource(MessageTransparent(case.transparent));
        world.resource_mut::<MessageOptions>().fixed = true;
        crate::events::message_boxes(
            &[amnezia_data::EventCommand {
                code: 10130,
                indent: 0,
                string: if case.face {
                    "Ron".into()
                } else {
                    String::new()
                },
                params: vec![6, 0, 0],
            }],
            &mut world.resource_mut::<Dialogue>().face,
        );
        if case.digits == 0 {
            let mut choice = world.resource_mut::<Choice>();
            choice.open(labels(index), 0, 0);
            choice.cursor = choice.options.len() - 1;
        } else {
            world.resource_mut::<InputNumber>().open(case.digits, 76);
        }
    }
    if age < 16 {
        return None;
    }
    let phase = usize::from(world.resource::<Clock>().source_x(0, case.digits > 0) == 96.0);
    let capture = index * 2 + phase;
    let mut probe = world.resource_mut::<Probe>();
    if probe.captured & (1 << capture) != 0 {
        return None;
    }
    probe.captured |= 1 << capture;
    Some(LABELS[capture])
}

pub(crate) struct Snapshot {
    pixels: Vec<[u8; 4]>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let capture = LABELS.iter().position(|candidate| *candidate == label)?;
    let index = capture / 2;
    let case = case(index);
    let origin = if capture % 2 == 0 { 64 } else { 96 };
    let server = world.resource::<AssetServer>();
    let skin = server.load::<Image>("graphics/System/System.png");
    let face = server.load::<Image>(crate::assets::resolve_png("FaceSet", "Ron"));
    let images = world.resource::<Assets<Image>>();
    let skin = images.get(&skin).unwrap();
    let left = if case.face { 84 } else { 12 };
    let runs = if case.digits == 0 {
        labels(index)
            .iter()
            .enumerate()
            .map(|(row, text)| Run::new(text, left, 2 + row as i32 * 16, DEFAULT))
            .collect()
    } else {
        (0..case.digits)
            .map(|digit| Run::new("0", left + digit as i32 * 12, 2, DEFAULT))
            .collect()
    };
    let glyphs = world.resource::<BitmapFont>().render(
        &PixelText {
            size: UVec2::new(304, 64),
            runs,
        },
        skin,
    );
    let offset = if case.face { 72 } else { 0 };
    let selection = if case.digits == 0 {
        (
            10 + offset,
            8 + if index == 2 { 32 } else { 48 },
            300 - offset,
            origin,
        )
    } else {
        (16 + offset + (case.digits - 1) * 12, 8, 14, origin)
    };
    Some(Snapshot {
        pixels: pixels::reference(
            skin,
            &glyphs,
            case.face.then(|| (images.get(&face).unwrap(), 6)),
            case.top,
            case.transparent,
            Some(selection),
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
                "message prompt ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "message prompt: 76800 original frame, portrait, bitmap text and cursor pixels verified"
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.captured, (1 << LABELS.len()) - 1);
    assert_eq!(probe.pixels.load(Ordering::Relaxed), LABELS.len());
    assert!(!world.resource::<Choice>().active());
    assert!(!world.resource::<InputNumber>().active());
}
