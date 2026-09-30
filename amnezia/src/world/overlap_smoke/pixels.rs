use super::*;
use crate::player::Player;
use crate::tiles;

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checked: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !matches!(
        label,
        "overlap-original-book-blocked" | "overlap-original-book-collected"
    ) {
        return None;
    }
    let camera = world
        .query_filtered::<&GlobalTransform, With<super::super::MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    let (hero, transform, visible) = world
        .query::<(&Player, &GlobalTransform, &InheritedVisibility)>()
        .single(world)
        .unwrap();
    assert_eq!(
        hero.tile(),
        if label.ends_with("-collected") {
            (10, 7)
        } else {
            HERO
        }
    );
    assert!(visible.get());
    assert_eq!(hero.charset, "Chara1");
    assert_eq!(hero.index, 0);
    let source = tiles::charset_source(0, hero.dir, hero.frame);
    let point = transform.translation() - camera;
    let left = (160.0 + point.x - 12.0).round() as u32;
    let top = (120.0 - point.y - 16.0).round() as u32;
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("CharSet", "Chara1"));
    let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let mut pixels = Vec::new();
    let mut head = 0;
    for y in 0..32 {
        for x in 0..24 {
            let pixel = image
                .get_color_at(source.0 as u32 + x, source.1 as u32 + y)
                .unwrap()
                .to_srgba()
                .to_u8_array();
            if pixel[3] == 255 {
                head += usize::from(y < 16);
                pixels.push((left + x, top + y, pixel));
            }
        }
    }
    assert!(
        head > 50 && pixels.len() > 180,
        "Ron's full head and body must be sampled"
    );
    Some(Snapshot {
        pixels,
        checked: world.resource::<Probe>().pictures.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            assert_eq!(
                crate::display::smoke::pixel_at(image, x, y),
                expected,
                "Ron's unobscured cave sprite at ({x},{y})"
            );
        }
        self.checked.fetch_add(1, Ordering::Relaxed);
        info!(
            "original ghost/book scene: all {} opaque Ron pixels, including his head, verified",
            self.pixels.len()
        );
    }
}
