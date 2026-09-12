//! FlashScreen uses a 60 Hz decay and expands its five-bit channels with ×8.

use bevy::prelude::*;

pub(crate) mod smoke;

/// The current level is fractional; only the rendered byte is truncated.
pub(super) struct Flashing {
    rgb: [i32; 3],
    level: f64,
    frames_left: u32,
    fraction: f64,
}

impl Flashing {
    pub(super) fn new(r: i32, g: i32, b: i32, intensity: i32, secs: f32) -> Self {
        Self {
            rgb: [r, g, b],
            level: f64::from(intensity),
            frames_left: (secs * 60.0).round().max(0.0) as u32,
            fraction: 0.0,
        }
    }

    pub(super) fn step(&mut self, dt: f32) -> bool {
        self.fraction += f64::from(dt) * 60.0;
        while self.fraction + 1e-6 >= 1.0 && self.frames_left > 0 {
            self.level -= self.level / f64::from(self.frames_left);
            self.frames_left -= 1;
            self.fraction = (self.fraction - 1.0).max(0.0);
        }
        if self.frames_left == 0 {
            self.level = 0.0;
        }
        self.frames_left > 0
    }

    pub(super) fn color(&self) -> Color {
        let [r, g, b] = self.rgb.map(|channel| (channel.clamp(0, 31) * 8) as u8);
        Color::srgba_u8(r, g, b, (self.level * 8.0).clamp(0.0, 248.0) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_five_bit_channels_expand_by_eight_including_alpha() {
        for channel in 0..=31 {
            let color = Flashing::new(channel, 10, 5, channel, 1.0)
                .color()
                .to_srgba();
            assert_eq!(color.red, (channel * 8) as f32 / 255.0);
            assert_eq!(color.green, 80.0 / 255.0);
            assert_eq!(color.blue, 40.0 / 255.0);
            assert_eq!(color.alpha, (channel * 8) as f32 / 255.0);
        }
    }

    #[test]
    fn flash_color_does_not_age_between_logical_frames() {
        let mut flash = Flashing::new(31, 10, 5, 31, 0.1);
        let initial = flash.color();
        assert!(flash.step(1.0 / 144.0));
        assert_eq!(flash.color(), initial);
        assert!(flash.step(1.0 / 144.0));
        assert_eq!(flash.color(), initial);
        assert!(flash.step(1.0 / 144.0));
        assert_ne!(flash.color(), initial);
    }

    #[test]
    fn flash_alpha_decays_from_peak_to_zero() {
        let mut flash = Flashing::new(31, 31, 31, 30, 0.5);
        assert_eq!(flash.color().alpha(), 240.0 / 255.0);
        assert!(flash.step(0.25));
        assert_eq!(flash.color().alpha(), 120.0 / 255.0);
        assert!(!flash.step(0.25));
        assert_eq!(flash.color().alpha(), 0.0);
        assert!(!flash.step(0.5));
    }

    #[test]
    fn flash_alpha_scales_with_intensity() {
        let flash = Flashing::new(31, 31, 31, 20, 0.5);
        assert_eq!(flash.color().alpha(), 160.0 / 255.0);
    }

    #[test]
    fn zero_duration_and_zero_intensity_flashes_are_invisible_after_updating() {
        let mut flash = Flashing::new(31, 0, 31, 31, 0.0);
        assert!(!flash.step(1.0 / 60.0));
        assert_eq!(flash.color().alpha(), 0.0);
        let mut flash = Flashing::new(31, 0, 31, 0, 1.0);
        assert!(flash.step(0.5));
        assert_eq!(flash.color().alpha(), 0.0);
    }

    #[test]
    fn flash_decays_and_then_reports_spent() {
        let mut flash = Flashing::new(31, 31, 31, 31, 0.5);
        assert!(flash.step(0.25));
        assert!((flash.color().alpha() - 124.0 / 255.0).abs() <= 1.0 / 255.0);
        assert!(!flash.step(0.5));
    }

    #[test]
    fn original_decay_and_byte_truncation_are_identical_at_15_to_144_fps() {
        for fps in [15, 30, 60, 120, 144] {
            let mut flash = Flashing::new(31, 10, 5, 31, 0.5);
            let mut reference = vec![248];
            let mut level = 31.0_f64;
            for left in (1..=30).rev() {
                level -= level / f64::from(left);
                reference.push((level * 8.0) as u8);
            }
            let mut clock = crate::timing::GameFrames::default();
            for _ in 0..fps {
                clock.advance(1.0 / f64::from(fps));
                let running = flash.step(1.0 / fps as f32);
                let tick = clock.frame.min(30) as usize;
                assert_eq!(running, tick < 30);
                assert_eq!(flash.color().alpha(), f32::from(reference[tick]) / 255.0);
            }
        }
    }
}
