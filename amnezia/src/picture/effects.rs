use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Effect {
    pub mode: i32,
    pub strength: i32,
}

impl Effect {
    pub(super) fn from_params(params: &[i32]) -> Self {
        let mode = params.get(12).copied().unwrap_or(0);
        Self {
            mode,
            strength: if mode == 0 {
                0
            } else {
                params.get(13).copied().unwrap_or(0)
            },
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct EffectState {
    mode: i32,
    strength: f64,
    target: f64,
    rotation: f64,
    phase: u8,
}

impl EffectState {
    pub(super) fn valid(&self) -> bool {
        (0..=2).contains(&self.mode)
            && self.strength.is_finite()
            && self.target.is_finite()
            && self.rotation.is_finite()
    }

    pub(super) fn show(effect: Effect) -> Self {
        Self {
            mode: effect.mode,
            strength: f64::from(effect.strength),
            target: f64::from(effect.strength),
            ..default()
        }
    }

    pub(super) fn retarget(&mut self, effect: Effect) {
        // RM2000 cannot introduce an effect after an effect-free ShowPicture.
        if self.mode == 0 && self.target == 0.0 {
            return;
        }
        match (self.mode, effect.mode) {
            (0, 0) => {}
            (old, new) if old == new => self.target = f64::from(effect.strength),
            (1, 0) => self.mode = 0,
            (2, 0) => self.target = 0.0,
            (_, new) => {
                self.mode = new;
                self.strength = f64::from(effect.strength);
                self.target = self.strength;
            }
        }
    }

    pub(super) fn tick(&mut self, remaining: u32) {
        if self.mode == 0 && self.strength > 0.0 {
            let revolution = 256 / (self.strength as u32).max(1);
            if revolution < remaining || self.rotation > 0.0 {
                self.rotation = self.rotation % 256.0 + self.strength;
                if revolution >= remaining && self.rotation >= 256.0 {
                    self.rotation = 0.0;
                }
            }
        }
        if self.mode != 0 {
            self.strength += (self.target - self.strength) / (f64::from(remaining) + 1.0);
        }
        if self.mode == 1 {
            self.rotation += self.strength;
        } else if self.mode == 2 {
            self.phase = self.phase.wrapping_add(8);
        }
    }

    pub(super) fn angle(&self) -> f32 {
        if self.mode == 2 {
            0.0
        } else {
            (self.rotation % 256.0 * std::f64::consts::TAU / 256.0) as f32
        }
    }

    pub(super) fn wave(&self, size: Vec2, zoom: f32, center: Vec2) -> Option<Wave> {
        let depth = (self.strength * 2.0).trunc() as f32;
        if self.mode != 2 || depth == 0.0 || zoom <= 0.0 {
            return None;
        }
        let top_left = (center.trunc() - (size / 2.0).floor() * zoom).trunc();
        let drawn = (size * zoom).floor();
        let amplitude = 2.0 * zoom * depth;
        let padding = amplitude.abs().ceil();
        Some(Wave {
            size: Vec2::new(drawn.x + 2.0 * padding, drawn.y),
            center: top_left + drawn / 2.0,
            uniform: Vec4::new(
                f32::from(self.phase) * std::f32::consts::TAU / 256.0,
                amplitude,
                zoom,
                (-top_left.y).max(0.0),
            ),
        })
    }
}

pub(super) struct Wave {
    pub size: Vec2,
    pub center: Vec2,
    pub uniform: Vec4,
}

#[cfg(test)]
mod tests;
