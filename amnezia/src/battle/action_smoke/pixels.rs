use super::*;
use crate::font::bitmap::{BitmapFont, DEFAULT, Run};
use crate::windowskin::reference::Canvas;

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    if !label.starts_with("action-") {
        return None;
    }
    let trace = world.resource::<Trace>();
    let font = world.resource::<BitmapFont>();
    let system = world
        .resource::<AssetServer>()
        .load("graphics/System/System.png");
    let skin = world.resource::<Assets<Image>>().get(&system).unwrap();
    let mut canvas = Canvas::new(None);
    canvas.window(skin, (0, 160, 320, 80));
    canvas.clip = IRect::new(8, 168, 312, 232);
    for (row, line) in trace.lines.iter().enumerate() {
        let text = font.render(
            &PixelText {
                size: UVec2::new(300, 16),
                runs: vec![Run::new(line, 0, 0, DEFAULT)],
            },
            skin,
        );
        canvas.blit(&text, (8, 170 + row as i32 * 16), (0, 0, 300, 16));
    }
    let pixels = canvas.pixels();
    assert_eq!(pixels.len(), 320 * 80);
    Some(Snapshot {
        pixels,
        checks: trace.pixels.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "action message ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "battle action window: {} source-derived pixels verified",
            self.pixels.len()
        );
    }
}
