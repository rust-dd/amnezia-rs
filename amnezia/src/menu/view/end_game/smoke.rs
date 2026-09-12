use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
struct Checks(Arc<AtomicUsize>);

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let selected = match label {
        "title-return-menu" | "end-game-no-blink" => 1,
        "end-game-yes" => 0,
        _ => return None,
    };
    world.init_resource::<Checks>();
    assert!(world.resource::<MenuOpen>().0);
    assert_eq!(
        world.resource::<MenuState>().screen,
        MenuScreen::EndGame { cursor: selected }
    );
    let font = world.resource::<BitmapFont>();
    let layout = Layout::new(world.resource::<Terms>(), font);
    assert_eq!(layout.labels, ["Játék vége?", "Igen", "Nem"]);
    let handle = world
        .resource::<AssetServer>()
        .load("graphics/System/System.png");
    let skin = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let background = skin.get_color_at(0, 32).unwrap().to_srgba().to_u8_array();
    let rectangles = [
        layout.rect(EndWindow::Help),
        layout.rect(EndWindow::Commands),
    ];
    let mut pixels = Vec::new();
    for y in 0..240 {
        for x in 0..320 {
            if !rectangles.iter().any(|&(left, top, width, height)| {
                x >= left && x < left + width as i32 && y >= top && y < top + height as i32
            }) {
                pixels.push((x as u32, y as u32, background));
            }
        }
    }
    for &(x, y, width, height) in &rectangles {
        super::super::smoke::border(
            &mut pixels,
            skin,
            (x as u32, y as u32, width, height),
            32,
            false,
        );
    }
    let source = world.resource::<Clock>().source_x() as u32;
    assert_eq!(source, if label == "title-return-menu" { 64 } else { 96 });
    let (x, y, width, _) = rectangles[1];
    super::super::smoke::border(
        &mut pixels,
        skin,
        (
            x as u32 + 4,
            y as u32 + 8 + selected as u32 * 16,
            width - 8,
            16,
        ),
        source,
        true,
    );
    for (index, label) in layout.labels.iter().enumerate() {
        let (x, y, width, _) = rectangles[usize::from(index != 0)];
        let text = font.render(
            &PixelText {
                size: UVec2::new(width - 16, 16),
                runs: vec![Run::new(label, 0, 0, DEFAULT)],
            },
            skin,
        );
        for row in 0..16 {
            for column in 0..width - 16 {
                super::super::smoke::sample(
                    &mut pixels,
                    &text,
                    (
                        x as u32 + 8 + column,
                        y as u32 + 10 + u32::from(index == 2) * 16 + row,
                    ),
                    (column, row),
                );
            }
        }
    }
    Some(Snapshot {
        pixels,
        checks: world.resource::<Checks>().0.clone(),
    })
}

pub(crate) fn assert_cancelled(world: &World) {
    assert!(world.resource::<MenuOpen>().0);
    let state = world.resource::<MenuState>();
    assert_eq!(state.screen, MenuScreen::Command);
    assert_eq!(state.cursor, 4);
    assert!(!world.resource::<crate::title::TitleActive>().0);
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "end game ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "end game: {} original background, frame, cursor and text pixels verified",
            self.pixels.len()
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().0.load(Ordering::Relaxed), 3);
}
