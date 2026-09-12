use super::*;

pub(super) fn expected(world: &World, opened: u32) -> Vec<[u8; 4]> {
    let assets = world.resource::<AssetServer>();
    let images = world.resource::<Assets<Image>>();
    let title = assets.load(crate::assets::resolve_png("Title", "Title"));
    let title = images.get(&title).unwrap();
    assert_eq!(title.size(), UVec2::new(320, 240));
    let system = assets.load("graphics/System/System.png");
    let system = images.get(&system).unwrap();
    let font = world.resource::<BitmapFont>();
    let layout = Layout::new(world.resource::<Terms>(), font);
    assert_eq!(layout.width, 64);
    assert_eq!(layout.labels, ["Új játék", "Betöltés", "Kilépés"]);
    let mut pixels = vec![[0, 0, 0, 255]; 320 * 240];
    for y in 0..240 {
        for x in 0..320 {
            put(&mut pixels, x, y, rgba(title, x, y));
        }
    }
    let height = opened * 8;
    let first_row = 180 - height / 2;
    for y in first_row..first_row + height {
        for x in 128..192 {
            put(
                &mut pixels,
                x,
                y,
                rgba(system, (x - 128) / 2, (y - 148) / 2),
            );
        }
    }
    let edge = (height / 2).min(8);
    for dy in 0..edge {
        for x in 0..64 {
            let sx = 32 + source(x, 64);
            put(&mut pixels, 128 + x, first_row + dy, rgba(system, sx, dy));
            put(
                &mut pixels,
                128 + x,
                first_row + height - edge + dy,
                rgba(system, sx, 32 - edge + dy),
            );
        }
    }
    for y in first_row + edge..first_row + height - edge {
        for dx in 0..8 {
            let sy = 8 + (y - 148) % 16;
            put(&mut pixels, 128 + dx, y, rgba(system, 32 + dx, sy));
            put(&mut pixels, 184 + dx, y, rgba(system, 56 + dx, sy));
        }
    }
    if opened == 8 {
        let selected = world.resource::<TitleState>().cursor as u32;
        let origin = world.resource::<clock::Clock>().source_x() as u32;
        for y in 0..16 {
            for x in 0..56 {
                put(
                    &mut pixels,
                    132 + x,
                    156 + selected * 16 + y,
                    rgba(system, origin + source(x, 56), source(y, 16)),
                );
            }
        }
        let has_save = crate::save::save_slot_exists();
        for (row, label) in layout.labels.iter().enumerate() {
            let text = font.render(
                &PixelText {
                    size: UVec2::new(48, 16),
                    runs: vec![Run::new(
                        label,
                        0,
                        0,
                        if row != CONTINUE || has_save {
                            DEFAULT
                        } else {
                            DISABLED
                        },
                    )],
                },
                system,
            );
            for y in 0..16 {
                for x in 0..48 {
                    put(
                        &mut pixels,
                        136 + x,
                        158 + row as u32 * 16 + y,
                        rgba(&text, x, y),
                    );
                }
            }
        }
    }
    pixels
}

fn source(position: u32, length: u32) -> u32 {
    if position < 8 {
        position
    } else if position >= length - 8 {
        32 - (length - position)
    } else {
        8 + position % 16
    }
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn put(pixels: &mut [[u8; 4]], x: u32, y: u32, source: [u8; 4]) {
    let target = &mut pixels[(y * 320 + x) as usize];
    let alpha = u32::from(source[3]);
    for i in 0..3 {
        target[i] =
            ((u32::from(source[i]) * alpha + u32::from(target[i]) * (255 - alpha)) / 255) as u8;
    }
}
