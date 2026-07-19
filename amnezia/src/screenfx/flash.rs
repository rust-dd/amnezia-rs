//! The brief screen flash (`FlashScreen` 11040): a full-screen colour that decays
//! linearly to nothing over its duration. RM2000 gives the colour and intensity
//! in 0..31 channels; [`Flashing`] holds one live flash and paints the fullscreen
//! UI overlay.

use bevy::prelude::*;

/// One decaying flash: its RM2000 0..31 colour and intensity, elapsed time, and
/// total duration.
pub(super) struct Flashing {
    r: i32,
    g: i32,
    b: i32,
    intensity: i32,
    elapsed: f32,
    secs: f32,
}

impl Flashing {
    /// Begin a flash of `r,g,b`/`intensity` (0..31) lasting `secs`.
    pub(super) fn new(r: i32, g: i32, b: i32, intensity: i32, secs: f32) -> Self {
        Self {
            r,
            g,
            b,
            intensity,
            elapsed: 0.0,
            secs,
        }
    }

    /// Advance by `dt` seconds; returns `false` once the flash has fully decayed.
    pub(super) fn step(&mut self, dt: f32) -> bool {
        self.elapsed += dt;
        self.elapsed < self.secs
    }

    /// The overlay colour for the current point in the decay.
    pub(super) fn color(&self) -> Color {
        let (r, g, b) = flash_color(self.r, self.g, self.b);
        Color::srgba(
            r,
            g,
            b,
            flash_alpha(self.intensity, self.elapsed, self.secs),
        )
    }
}

/// The flash overlay's RGB (0..1) from RM2000 0..31 channels.
fn flash_color(r: i32, g: i32, b: i32) -> (f32, f32, f32) {
    (
        (r as f32 / 31.0).clamp(0.0, 1.0),
        (g as f32 / 31.0).clamp(0.0, 1.0),
        (b as f32 / 31.0).clamp(0.0, 1.0),
    )
}

/// The flash alpha, decaying linearly from `intensity/31` at `elapsed == 0` to 0
/// at `elapsed == secs`.
fn flash_alpha(intensity: i32, elapsed: f32, secs: f32) -> f32 {
    if secs <= 0.0 {
        return 0.0;
    }
    let peak = (intensity as f32 / 31.0).clamp(0.0, 1.0);
    (peak * (1.0 - elapsed / secs)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_alpha_decays_from_peak_to_zero() {
        assert!((flash_alpha(31, 0.0, 0.5) - 1.0).abs() < 1e-6);
        assert!((flash_alpha(31, 0.25, 0.5) - 0.5).abs() < 1e-6);
        assert_eq!(flash_alpha(31, 0.5, 0.5), 0.0);
        assert_eq!(flash_alpha(31, 1.0, 0.5), 0.0);
    }

    #[test]
    fn flash_alpha_scales_with_intensity() {
        assert!((flash_alpha(20, 0.0, 0.5) - 20.0 / 31.0).abs() < 1e-6);
    }

    #[test]
    fn flash_color_normalises_0_31_channels() {
        assert_eq!(flash_color(31, 0, 31), (1.0, 0.0, 1.0));
    }

    #[test]
    fn flash_decays_and_then_reports_spent() {
        let mut flash = Flashing::new(31, 31, 31, 31, 0.5);
        assert!(flash.step(0.25));
        assert!((flash.color().alpha() - 0.5).abs() < 1e-6);
        assert!(!flash.step(0.5));
    }
}
