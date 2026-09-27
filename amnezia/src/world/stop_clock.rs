use crate::dialogue::MessageOptions;
use crate::interpreter::RunningEvent;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

// Legacy random delays could be 307.2 frames before conversion to whole updates.
const MAXIMUM: u32 = 308;

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct StopClock {
    pub(crate) count: u32,
    pub(crate) maximum: u32,
}

impl StopClock {
    pub(crate) fn active(self) -> bool {
        self.count < self.maximum
    }

    pub(crate) fn valid(self) -> bool {
        self.count <= i32::MAX as u32 && self.maximum <= MAXIMUM
    }

    pub(crate) fn advance(&mut self, moving: bool, forced: bool, allowed: bool) {
        if moving {
            self.count = 0;
        } else if self.count == 0 || forced || allowed {
            self.count = self.count.saturating_add(1).min(i32::MAX as u32);
        }
    }

    pub(crate) fn from_legacy(seconds: f32, maximum: u32) -> Self {
        // Legacy countdowns lack the elapsed count and may contain sub-frame residue.
        let remaining = (f64::from(seconds) * 60.0 - 0.00001)
            .ceil()
            .clamp(0.0, f64::from(MAXIMUM)) as u32;
        let maximum = maximum.max(remaining);
        Self {
            count: maximum - remaining,
            maximum,
        }
    }
}

pub(crate) fn step(frequency: u32) -> u32 {
    threshold(frequency, 9)
}

pub(crate) fn turn(frequency: u32) -> u32 {
    threshold(frequency, 8)
}

pub(crate) fn wait(frequency: u32) -> u32 {
    20 + turn(frequency)
}

fn threshold(frequency: u32, shift: u32) -> u32 {
    let frequency = frequency.clamp(1, 8);
    if frequency == 8 {
        0
    } else {
        1 << (shift - frequency)
    }
}

pub(crate) fn legacy_empty(value: &f32) -> bool {
    *value == 0.0
}

pub(crate) fn legacy_valid(value: f32) -> bool {
    value.is_finite() && value <= MAXIMUM as f32 / 60.0
}

#[derive(SystemParam)]
pub(crate) struct StopGates<'w> {
    running: Option<Res<'w, RunningEvent>>,
    options: Option<Res<'w, MessageOptions>>,
}

impl StopGates<'_> {
    pub(crate) fn advances(&self, event_id: Option<u32>) -> bool {
        self.running.as_ref().is_none_or(|running| {
            (self
                .options
                .as_ref()
                .is_some_and(|options| options.continue_events)
                || !running.active())
                && event_id.is_none_or(|id| !running.event_paused(id))
        })
    }
}

#[cfg(test)]
mod tests;
