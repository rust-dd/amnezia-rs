use super::*;
use amnezia_data::EventCommand;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod pixels;

#[derive(Resource, Default)]
struct Probe(Arc<AtomicUsize>);

struct Case {
    top: u32,
    face: bool,
    transparent: bool,
    text: String,
}

fn case(index: u32) -> Case {
    Case {
        top: match index { 1 => 0, 2 => 80, _ => 160 },
        face: matches!(index, 0 | 2),
        transparent: index == 2,
        text: match index {
            0 => "Hát... hol vagyok?\nÁrvíztűrő tükörfúrógép.\nŐrült éjszaka volt!\n0123456789 ÁÉÍÓÖŐÚÜŰ".into(),
            3 => format!("{}\nŐrült éjszaka volt!\nÁÉÍÓÖŐÚÜŰ\n0123456789", "W".repeat(70)),
            _ => "Ron\nÁrvíztűrő tükörfúrógép.\nÁÉÍÓÖŐÚÜŰ\n0123456789".into(),
        },
    }
}

fn command(code: u32, string: &str, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent: 0,
        string: string.into(),
        params,
    }
}

fn start(world: &mut World, index: u32) {
    let case = case(index);
    assert!(!world.resource::<Dialogue>().active);
    let mut running = world.resource_mut::<crate::interpreter::RunningEvent>();
    assert!(!running.active());
    running.start(
        0,
        vec![
            command(
                10120,
                "",
                vec![i32::from(case.transparent), case.top as i32 / 80, 0, 0],
            ),
            command(10130, if case.face { "Ron" } else { "" }, vec![0, 0, 0]),
            command(10110, &case.text, Vec::new()),
        ],
    );
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        260 => {
            world.init_resource::<Probe>();
        }
        400 | 520 | 680 => world.resource_mut::<Dialogue>().close(),
        420 => start(world, 1),
        540 => start(world, 2),
        700 => start(world, 3),
        _ => {}
    }
    match frame {
        500 => Some("dialogue-font-no-face"),
        650 => Some("dialogue-font-transparent"),
        820 => Some("dialogue-font-clipped"),
        _ => None,
    }
}

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let index = match label {
        "font-early" => 0,
        "dialogue-font-no-face" => 1,
        "dialogue-font-transparent" => 2,
        "dialogue-font-clipped" | "font" => 3,
        _ => return None,
    };
    let case = case(index);
    let dialogue = world.resource::<Dialogue>();
    assert!(dialogue.active);
    assert!(dialogue.reveal.as_ref().unwrap().is_complete());
    assert_eq!(world.resource::<MessageTransparent>().0, case.transparent);
    let expected_text = PixelText {
        size: UVec2::new(304, 64),
        runs: vec![Run::new(
            &case.text,
            if case.face { 72 } else { 0 },
            2,
            DEFAULT,
        )],
    };
    let (text, visible) = world
        .query_filtered::<(&PixelText, &InheritedVisibility), With<DialogueText>>()
        .single(world)
        .unwrap();
    assert_eq!(*text, expected_text);
    assert!(visible.get());
    let node = world
        .query_filtered::<&Node, With<DialoguePanel>>()
        .single(world)
        .unwrap();
    assert_eq!(
        node.top,
        if case.top == 160 {
            Val::Auto
        } else {
            Val::Px(case.top as f32 * 3.0)
        }
    );
    assert_eq!(
        node.bottom,
        if case.top == 160 {
            Val::Px(0.0)
        } else {
            Val::Auto
        }
    );
    let arrow = world
        .query_filtered::<&InheritedVisibility, With<DialogueArrow>>()
        .single(world)
        .unwrap()
        .get();
    let server = world.resource::<AssetServer>();
    let system = server.load::<Image>("graphics/System/System.png");
    let portrait = server.load::<Image>(resolve_png("FaceSet", "Ron"));
    let images = world.resource::<Assets<Image>>();
    let skin = images.get(&system).unwrap();
    let glyphs = world
        .resource::<crate::font::bitmap::BitmapFont>()
        .render(&expected_text, skin);
    let portrait = case.face.then(|| images.get(&portrait).unwrap());
    Some(Snapshot {
        pixels: pixels::reference(skin, &glyphs, portrait, &case, arrow),
        checks: world.resource::<Probe>().0.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "dialogue ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "dialogue font/frame: {} original pixels verified",
            self.pixels.len()
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Probe>().0.load(Ordering::Relaxed), 5);
}
