use super::model::{Drop, HEIGHT, WIDTH};

pub(super) fn streak() -> [(i32, i32); 24] {
    std::array::from_fn(|y| (5 - y as i32 / 4, y as i32))
}

pub(super) fn surface(drops: &[Drop], strength: i32) -> Vec<u8> {
    let strength = strength.clamp(0, 2);
    let mut pixels = vec![0; (WIDTH * HEIGHT) as usize];
    for drop in drops.iter().take(super::super::particle_count(strength)) {
        if drop.life > 12 {
            continue;
        }
        let alpha = ((5 + strength) * i32::from(drop.life)) as u8;
        let columns = if drop.x + 6 > WIDTH { 2 } else { 1 };
        let rows = if drop.y + 24 > HEIGHT { 2 } else { 1 };
        for row in 0..rows {
            for column in 0..columns {
                for (sx, sy) in streak() {
                    let x = drop.x + sx - column * WIDTH;
                    let y = drop.y + sy - row * HEIGHT;
                    // EdgeMirrorBlit copies right/bottom overflow only once.
                    if !(0..WIDTH).contains(&x) || !(0..HEIGHT).contains(&y) {
                        continue;
                    }
                    let pixel = &mut pixels[(y * WIDTH + x) as usize];
                    let product = u32::from(*pixel) * u32::from(255 - alpha) + 128;
                    *pixel = alpha + ((product + (product >> 8)) >> 8) as u8;
                }
            }
        }
    }
    pixels
}

pub(super) fn viewport(surface: &[u8], offset: [i32; 2], color: [u8; 3], pixels: &mut [u8]) {
    for y in 0..240 {
        let sy = (y + offset[1]).rem_euclid(HEIGHT);
        for x in 0..WIDTH {
            let sx = (x + offset[0]).rem_euclid(WIDTH);
            let alpha = surface[(sy * WIDTH + sx) as usize];
            let pixel = &mut pixels[((y * WIDTH + x) * 4) as usize..][..4];
            pixel.copy_from_slice(&[color[0], color[1], color[2], alpha]);
        }
    }
}

#[cfg(test)]
mod tests;
