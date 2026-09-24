use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const LABELS: [&str; 8] = [
    "battle-encounter-two",
    "battle-menus-early",
    "battle-commands",
    "battle-skills",
    "battle-skills-scrolled",
    "battle-target",
    "battle-ally-target",
    "battle-resized",
];

#[derive(Resource, Default)]
struct Checks(Arc<AtomicUsize>);

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !LABELS.contains(&label) && !navigation::smoke::LABELS.contains(&label) {
        return None;
    }
    world.init_resource::<Checks>();
    let active = Panel::active(world.resource::<Battle>());
    let cursors = world
        .query::<(&view::Cursor, &Node, &InheritedVisibility)>()
        .iter(world)
        .filter(|(cursor, _, visible)| visible.get() && active == Some(cursor.0))
        .map(|(cursor, node, _)| (cursor.0, node.clone()))
        .collect::<Vec<_>>();
    let battle = world.resource::<Battle>();
    let windows = world.resource::<motion::CommandWindows>();
    let lists = world.resource::<navigation::Windows>();
    let handle = world
        .resource::<AssetServer>()
        .load("graphics/System/System.png");
    let skin = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let mut pixels = super::pixels::compose(world);
    let window_pixels = pixels.len();
    let native = |value| {
        if let Val::Px(value) = value {
            (value / 3.0) as i32
        } else {
            panic!("cursor must use native pixels")
        }
    };
    for (panel, node) in cursors {
        let (window_x, window_y, _, _) = layout::rectangle(panel, battle).unwrap();
        let left = windows.x(panel).unwrap_or(window_x) as i32 + native(node.left);
        let top = window_y as i32 + native(node.top);
        let width = native(node.width);
        let height = native(node.height);
        let origin = lists.get(panel).cursor_x() as u32;
        for y in 0..height {
            for x in 0..width {
                if x != 0 && y != 0 && x + 1 != width && y + 1 != height {
                    continue;
                }
                if !(0..320).contains(&(left + x)) || !(0..240).contains(&(top + y)) {
                    continue;
                }
                let source = |p, length| {
                    if p < 8 {
                        p
                    } else if p >= length - 8 {
                        24 + p - (length - 8)
                    } else {
                        8 + (p - 8) % 16
                    }
                };
                let rgba = skin
                    .get_color_at(origin + source(x, width) as u32, source(y, height) as u32)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                let exposed = pixels[..window_pixels]
                    .binary_search_by_key(&((top + y) as u32, (left + x) as u32), |pixel| {
                        (pixel.1, pixel.0)
                    })
                    .is_ok_and(|index| pixels[index].2 == rgba);
                if rgba[3] == 255 && exposed {
                    pixels.push(((left + x) as u32, (top + y) as u32, rgba));
                }
            }
        }
    }
    assert!(
        window_pixels >= 320 * 80,
        "{label} has no complete lower window"
    );
    if active.is_some() {
        assert!(
            pixels.len() > window_pixels + 100,
            "{label} has no visible cursor border"
        );
    }
    Some(Snapshot {
        pixels,
        checks: world.resource::<Checks>().0.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "battle cursor ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "battle windows: {} native background, text and cursor pixels verified",
            self.pixels.len()
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(
        world.resource::<Checks>().0.load(Ordering::Relaxed),
        LABELS.len() + navigation::smoke::LABELS.len()
    );
}
