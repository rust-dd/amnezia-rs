use super::Vehicles;
use crate::tiles::DIR_LEFT;
use crate::world::{Character, MapData};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct AirshipFlight {
    ascent: u16,
    descent: u16,
    fraction: f64,
}

impl AirshipFlight {
    pub(super) fn ascend(&mut self) {
        self.ascent = 256;
        self.descent = 0;
    }

    pub(super) fn descend(&mut self) {
        self.ascent = 0;
        self.descent = 256;
    }

    fn active(&self) -> bool {
        self.ascent != 0 || self.descent != 0
    }

    fn frames(&mut self, dt: f32) -> u32 {
        self.fraction += f64::from(dt.max(0.0)) * 60.0;
        let frames = (self.fraction + 1e-6).floor() as u32;
        self.fraction = (self.fraction - f64::from(frames)).max(0.0);
        frames
    }

    fn step(&mut self) -> bool {
        if self.ascent > 0 {
            self.ascent = self.ascent.saturating_sub(8);
        } else if self.descent > 0 {
            self.descent = self.descent.saturating_sub(8);
            return self.descent == 0;
        }
        false
    }

    pub(super) fn valid(&self) -> bool {
        self.ascent <= 256
            && self.descent <= 256
            && (self.ascent == 0 || self.descent == 0)
            && (0.0..1.0).contains(&self.fraction)
    }

    pub(super) fn sanitize(&mut self) {
        self.ascent = self.ascent.min(256);
        self.descent = if self.ascent > 0 {
            0
        } else {
            self.descent.min(256)
        };
        if !self.fraction.is_finite() || !(0.0..1.0).contains(&self.fraction) {
            self.fraction = 0.0;
        }
    }
}

impl Vehicles {
    pub(crate) fn airship_ascent_remaining(&self) -> u16 {
        self.save.airship_flight.ascent
    }

    pub(crate) fn airship_transitioning(&self) -> bool {
        self.save.riding == Some(2) && self.save.airship_flight.active()
    }

    pub(crate) fn airship_altitude(&self) -> f32 {
        if self.save.riding != Some(2) {
            return 0.0;
        }
        let flight = &self.save.airship_flight;
        let subpixels = if flight.ascent > 0 {
            256 - flight.ascent
        } else if flight.descent > 0 {
            flight.descent
        } else {
            256
        };
        f32::from(subpixels / 16)
    }

    pub(super) fn can_land_airship(
        &self,
        data: &MapData,
        blocked: &impl Fn(i32, i32) -> bool,
    ) -> bool {
        let (x, y) = self.save.vehicles[2].tile();
        data.airship_landing_tile(x, y)
            && !self.save.vehicles[..2]
                .iter()
                .any(|other| other.definition.map_id == data.map_id && other.tile() == (x, y))
            && !blocked(x, y)
    }

    pub(super) fn advance_flight(
        &mut self,
        dt: f32,
        data: &MapData,
        blocked: impl Fn(i32, i32) -> bool,
    ) -> bool {
        if self.save.riding != Some(2) {
            return false;
        }
        for _ in 0..self.save.airship_flight.frames(dt) {
            if self.save.airship_flight.step() {
                if self.can_land_airship(data, &blocked) {
                    self.save.vehicles[2].dir = DIR_LEFT;
                    self.save.riding = None;
                    return true;
                }
                self.save.airship_flight.ascend();
            }
        }
        false
    }
}

#[cfg(test)]
mod tests;
