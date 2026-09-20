use super::*;

pub(super) struct Canvas(pub Vec<[u8; 4]>);

impl Canvas {
    pub(super) fn new() -> Self {
        Self(vec![[0, 0, 0, 255]; 320 * 240])
    }

    pub(super) fn window(
        &mut self,
        skin: &Image,
        (left, top, width, height): (u32, u32, u32, u32),
    ) {
        for y in 0..height {
            for x in 0..width {
                let mut pixel = rgba(skin, sample(x, width), sample(y, height));
                if x < 8 || y < 8 || x >= width - 8 || y >= height - 8 {
                    pixel = over(rgba(skin, 32 + tile(x, width), tile(y, height)), pixel);
                }
                self.0[((top + y) * 320 + left + x) as usize] = pixel;
            }
        }
    }

    pub(super) fn cursor(
        &mut self,
        skin: &Image,
        (left, top, width, height): (u32, u32, u32, u32),
        source: u32,
    ) {
        for y in 0..height {
            for x in 0..width {
                self.put(
                    left + x,
                    top + y,
                    rgba(skin, source + tile(x, width), tile(y, height)),
                );
            }
        }
    }

    pub(super) fn text(
        &mut self,
        font: &BitmapFont,
        skin: &Image,
        rect: (u32, u32, u32, u32),
        text: PixelText,
        offset: u32,
    ) {
        let image = font.render(&text, skin);
        let (left, top, width, height) = rect;
        self.blit(
            &image,
            (left, top),
            (0, offset, width, height.min(image.height() - offset)),
            false,
        );
    }

    pub(super) fn blit(
        &mut self,
        image: &Image,
        (left, top): (u32, u32),
        (sx, sy, width, height): (u32, u32, u32, u32),
        gray: bool,
    ) {
        for y in 0..height {
            for x in 0..width {
                let mut pixel = rgba(image, sx + x, sy + y);
                if gray {
                    let luma = ((19595 * u32::from(pixel[0])
                        + 38470 * u32::from(pixel[1])
                        + 7471 * u32::from(pixel[2]))
                        >> 16) as u8;
                    pixel[..3].fill(luma);
                }
                self.put(left + x, top + y, pixel);
            }
        }
    }

    fn put(&mut self, x: u32, y: u32, foreground: [u8; 4]) {
        let pixel = &mut self.0[(y * 320 + x) as usize];
        *pixel = over(foreground, *pixel);
    }
}

fn sample(position: u32, size: u32) -> u32 {
    let step = (32 << 16) / size;
    ((2 * position + 1) * step / 2).saturating_sub(1) >> 16
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
