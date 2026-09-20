use super::*;
mod pixels;
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
    let source = world.resource::<Clock>().source_x() as u32;
    assert_eq!(source, if label == "title-return-menu" { 64 } else { 96 });
    let pixels = pixels::compose(skin, font, &layout, selected, source);
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
