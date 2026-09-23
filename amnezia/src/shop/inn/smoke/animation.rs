use super::*;

pub(super) const LABELS: [&str; 14] = [
    "inn-open-0",
    "inn-open-1",
    "inn-open-2",
    "inn-open-3",
    "inn-open-4",
    "inn-open-5",
    "inn-open-6",
    "inn-close-1",
    "inn-close-2",
    "inn-close-3",
    "inn-close-4",
    "inn-close-5",
    "inn-close-6",
    "inn-close-7",
];
const MESSAGE: [u32; 14] = [0, 5, 11, 17, 22, 28, 34, 34, 28, 22, 17, 11, 5, 0];
const GOLD: [u32; 14] = [0, 2, 4, 6, 9, 11, 13, 13, 11, 9, 6, 4, 2, 0];

pub(super) fn capture(world: &mut World, step: Step, case: usize) -> Option<&'static str> {
    if case != 0 {
        return None;
    }
    let dialogue = world.resource::<Dialogue>();
    let half = dialogue.lifecycle.message.half_height(80);
    let range = if step == Step::Typing && dialogue.active && !dialogue.lifecycle.message.ready() {
        0..7
    } else if matches!(step, Step::Ready | Step::Exit) && !dialogue.active && dialogue.busy() {
        7..14
    } else {
        return None;
    };
    let index = range
        .into_iter()
        .find(|index| MESSAGE[*index] == half)
        .unwrap();
    assert_eq!(dialogue.lifecycle.gold.half_height(32), GOLD[index]);
    let mut probe = world.resource_mut::<Probe>();
    let fresh = probe.animation & (1 << index) == 0;
    probe.animation |= 1 << index;
    fresh.then_some(LABELS[index])
}

pub(super) fn reference(world: &World, label: &str) -> Option<Vec<[u8; 4]>> {
    let index = LABELS.iter().position(|expected| *expected == label)?;
    let server = world.resource::<AssetServer>();
    let images = world.resource::<Assets<Image>>();
    let skin = server.load::<Image>("graphics/System/System.png");
    let skin = images.get(&skin).unwrap();
    let mut pixels = vec![[0, 0, 0, 255]; 320 * 240];
    paint(&mut pixels, skin, (0, 160, 320, 80), MESSAGE[index]);
    paint(&mut pixels, skin, (232, 0, 88, 32), GOLD[index]);
    Some(pixels)
}

fn paint(pixels: &mut [[u8; 4]], skin: &Image, rect: (u32, u32, u32, u32), half: u32) {
    let (left, top, width, height) = rect;
    let first = height / 2 - half;
    let last = height / 2 + half;
    let sample =
        |p: u32, length: u32| (((2 * p + 1) * ((32 << 16) / length) / 2).saturating_sub(1)) >> 16;
    for y in first..last {
        for x in 0..width {
            let source = skin
                .get_color_at(sample(x, width), sample(y, height))
                .unwrap();
            pixels[((top + y) * 320 + left + x) as usize] = source.to_srgba().to_u8_array();
        }
    }
    for y in 0..half.min(8) {
        for x in 0..width {
            let sx = if x < 8 {
                32 + x
            } else if x >= width - 8 {
                64 - (width - x)
            } else {
                40 + x % 16
            };
            over(pixels, skin, (left + x, top + first + y), (sx, y));
            over(pixels, skin, (left + x, top + last - 1 - y), (sx, 31 - y));
        }
    }
    if half > 8 {
        for y in first + 8..last - 8 {
            for x in 0..8 {
                over(pixels, skin, (left + x, top + y), (32 + x, 8 + y % 16));
                over(
                    pixels,
                    skin,
                    (left + width - 8 + x, top + y),
                    (56 + x, 8 + y % 16),
                );
            }
        }
    }
}

fn over(pixels: &mut [[u8; 4]], skin: &Image, (x, y): (u32, u32), (sx, sy): (u32, u32)) {
    let source = skin.get_color_at(sx, sy).unwrap().to_srgba().to_u8_array();
    let pixel = &mut pixels[(y * 320 + x) as usize];
    for channel in 0..3 {
        pixel[channel] = ((u32::from(source[channel]) * u32::from(source[3])
            + u32::from(pixel[channel]) * (255 - u32::from(source[3])))
            / 255) as u8;
    }
}
