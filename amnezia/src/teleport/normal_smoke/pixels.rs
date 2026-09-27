use super::*;

pub(crate) struct Snapshot {
    pixels: Vec<[u8; 4]>,
    checked: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    if label != "normal-transfer-inn-closing" {
        return None;
    }
    Some(Snapshot {
        pixels: crate::shop::inn::smoke::animation_reference(world, "inn-close-1").unwrap(),
        checked: world.resource::<Probe>().pixels.clone(),
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
                "interrupted inn ({x}, {y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checked.fetch_add(1, Ordering::Relaxed);
        info!("interrupted inn: 76800 original closing-window pixels verified");
    }
}
