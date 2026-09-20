use super::*;

pub(super) fn compose(
    skin: &Image,
    font: &BitmapFont,
    layout: &Layout,
    selected: usize,
    source: u32,
) -> Vec<(u32, u32, [u8; 4])> {
    let mut canvas = vec![rgba(skin, 0, 32); 320 * 240];
    let rectangles = [
        layout.rect(EndWindow::Help),
        layout.rect(EndWindow::Commands),
    ];
    let texts = std::array::from_fn::<_, 3, _>(|index| {
        font.render(
            &PixelText {
                size: UVec2::new(layout.widths[usize::from(index != 0)] - 16, 16),
                runs: vec![Run::new(&layout.labels[index], 0, 0, DEFAULT)],
            },
            skin,
        )
    });
    for (window, (left, top, width, height)) in rectangles.into_iter().enumerate() {
        for y in 0..height {
            for x in 0..width {
                let mut pixel = rgba(skin, background(x, width), background(y, height));
                if x < 8 || y < 8 || x >= width - 8 || y >= height - 8 {
                    pixel = over(rgba(skin, 32 + tile(x, width), tile(y, height)), pixel);
                }
                let cy = 8 + selected as u32 * 16;
                if window == 1 && (4..width - 4).contains(&x) && (cy..cy + 16).contains(&y) {
                    pixel = over(
                        rgba(skin, source + tile(x - 4, width - 8), tile(y - cy, 16)),
                        pixel,
                    );
                }
                for (index, text) in texts.iter().enumerate() {
                    if usize::from(index != 0) != window {
                        continue;
                    }
                    let ty = 10 + u32::from(index == 2) * 16;
                    if (8..width - 8).contains(&x) && (ty..ty + 16).contains(&y) {
                        pixel = over(rgba(text, x - 8, y - ty), pixel);
                    }
                }
                canvas[(top as u32 + y) as usize * 320 + (left as u32 + x) as usize] = pixel;
            }
        }
    }
    canvas
        .into_iter()
        .enumerate()
        .map(|(index, pixel)| (index as u32 % 320, index as u32 / 320, pixel))
        .collect()
}

fn background(position: u32, size: u32) -> u32 {
    let scale = (32 << 16) / size;
    ((2 * position + 1) * scale / 2).saturating_sub(1) >> 16
}

fn tile(position: u32, size: u32) -> u32 {
    if position < 8 {
        position
    } else if position >= size - 8 {
        24 + position - (size - 8)
    } else {
        8 + (position - 8) % 16
    }
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn over(foreground: [u8; 4], background: [u8; 4]) -> [u8; 4] {
    let alpha = u32::from(foreground[3]);
    let mut pixel = [0, 0, 0, 255];
    for i in 0..3 {
        pixel[i] = ((u32::from(foreground[i]) * alpha + u32::from(background[i]) * (255 - alpha))
            / 255) as u8;
    }
    pixel
}
