use bevy::prelude::*;

/// Independent native-pixel reference used by the GPU window probes.
pub(crate) struct Canvas {
    pixels: Vec<Option<[u8; 4]>>,
    pub clip: IRect,
}

impl Canvas {
    pub fn new(background: Option<[u8; 4]>) -> Self {
        Self {
            pixels: vec![background; 320 * 240],
            clip: IRect::new(0, 0, 320, 240),
        }
    }

    pub fn window(&mut self, skin: &Image, rect: (i32, i32, u32, u32)) {
        let (left, top, width, height) = rect;
        for y in 0..height {
            for x in 0..width {
                self.put(
                    left + x as i32,
                    top + y as i32,
                    rgba(skin, background(x, width), background(y, height)),
                );
                if x < 8 || y < 8 || x + 8 >= width || y + 8 >= height {
                    self.put(
                        left + x as i32,
                        top + y as i32,
                        rgba(skin, 32 + tile(x, width), tile(y, height)),
                    );
                }
            }
        }
    }

    pub fn cursor(&mut self, skin: &Image, rect: (i32, i32, u32, u32), origin: u32) {
        let (left, top, width, height) = rect;
        for y in 0..height {
            for x in 0..width {
                self.put(
                    left + x as i32,
                    top + y as i32,
                    rgba(skin, origin + tile(x, width), tile(y, height)),
                );
            }
        }
    }

    pub fn blit(&mut self, image: &Image, target: (i32, i32), source: (u32, u32, u32, u32)) {
        let (sx, sy, width, height) = source;
        for y in 0..height {
            for x in 0..width {
                self.put(
                    target.0 + x as i32,
                    target.1 + y as i32,
                    rgba(image, sx + x, sy + y),
                );
            }
        }
    }

    fn put(&mut self, x: i32, y: i32, foreground: [u8; 4]) {
        if !(0..320).contains(&x)
            || !(0..240).contains(&y)
            || x < self.clip.min.x
            || x >= self.clip.max.x
            || y < self.clip.min.y
            || y >= self.clip.max.y
            || foreground[3] == 0
        {
            return;
        }
        let pixel = &mut self.pixels[(y * 320 + x) as usize];
        let background = pixel.unwrap_or([0; 4]);
        assert!(foreground[3] == 255 || background[3] == 255);
        let alpha = u32::from(foreground[3]);
        let mut color = [0, 0, 0, 255];
        for i in 0..3 {
            color[i] = ((u32::from(foreground[i]) * alpha
                + u32::from(background[i]) * (255 - alpha))
                / 255) as u8;
        }
        *pixel = Some(color);
    }

    pub fn pixels(self) -> Vec<(u32, u32, [u8; 4])> {
        self.pixels
            .into_iter()
            .enumerate()
            .filter_map(|(index, color)| {
                color.map(|color| (index as u32 % 320, index as u32 / 320, color))
            })
            .collect()
    }
}

pub(crate) fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
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
