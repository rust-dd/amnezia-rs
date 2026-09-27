use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Tracking {
    world: [f32; 2],
    display: [f32; 2],
    effects: [f32; 2],
    panorama: [f32; 2],
    bounded_panorama: [f32; 2],
    initial_display: [f32; 2],
}

impl Tracking {
    fn new(data: &MapData, position: Vec2, half_view: Vec2, effects: Option<Vec2>) -> Self {
        let corner = Vec2::from(data.tile_center(0, 0)) + Vec2::new(-8.0, 8.0);
        let raw = (position - corner) * Vec2::new(1.0, -1.0) - half_view;
        let display = normalize(data, raw, half_view).to_array();
        Self {
            world: position.to_array(),
            display,
            effects: effects.unwrap_or(position).to_array(),
            panorama: display,
            bounded_panorama: display,
            initial_display: display,
        }
    }

    pub(super) fn valid(&self) -> bool {
        self.world
            .iter()
            .chain(&self.display)
            .chain(&self.effects)
            .chain(&self.panorama)
            .chain(&self.bounded_panorama)
            .chain(&self.initial_display)
            .all(|value| value.is_finite())
    }
}

impl CameraPan {
    pub(crate) fn effects_position(&self) -> Option<Vec2> {
        self.position.map(|position| {
            self.tracking
                .as_ref()
                .filter(|state| state.world == position.to_array())
                .map_or(position, |state| Vec2::from_array(state.effects))
        })
    }

    pub(crate) fn panorama_position(&self, data: &MapData, looping: [bool; 2]) -> Option<Vec2> {
        let position = self.position?;
        let state = self
            .tracking
            .as_ref()
            .filter(|state| state.world == position.to_array())?;
        let mut display = Vec2::ZERO;
        for axis in 0..2 {
            display[axis] = if looping[axis] {
                state.panorama[axis]
            } else if [data.loops_x(), data.loops_y()][axis] {
                state.initial_display[axis]
            } else {
                state.bounded_panorama[axis]
            };
        }
        Some(display)
    }

    pub(super) fn initialize_position(&mut self, data: &MapData, position: Vec2, half_view: Vec2) {
        let effects = self
            .tracking
            .as_ref()
            .map(|state| Vec2::from_array(state.effects));
        self.tracking = Some(Tracking::new(data, position, half_view, effects));
        self.position = Some(position);
    }

    pub(super) fn scroll_to(&mut self, data: &MapData, position: Vec2, half_view: Vec2) {
        let before = self.position.unwrap_or(position);
        if self
            .tracking
            .as_ref()
            .is_none_or(|state| state.world != before.to_array())
        {
            self.tracking = Some(Tracking::new(data, before, half_view, None));
        }
        let state = self.tracking.as_mut().unwrap();
        let moved = position - before;
        let map_delta = moved * Vec2::new(1.0, -1.0);
        let period = Vec2::new(data.width as f32, data.height as f32) * TILE;
        for axis in 0..2 {
            state.display[axis] += map_delta[axis];
            if [data.loops_x(), data.loops_y()][axis] {
                state.display[axis] %= period[axis];
            }
            if map_delta[axis] != 0.0 {
                state.panorama[axis] += map_delta[axis];
                state.bounded_panorama[axis] = state.display[axis];
            }
            state.effects[axis] += moved[axis];
        }
        state.world = position.to_array();
        self.position = Some(position);
    }

    pub(super) fn round_jump(&mut self, data: &MapData, half_view: Vec2) {
        let position = self.position.unwrap();
        self.scroll_to(data, position, half_view);
        let state = self.tracking.as_mut().unwrap();
        let display = Vec2::from_array(state.display);
        let rounded = Vec2::new(
            (display.x / TILE).round_ties_even(),
            (display.y / TILE).round_ties_even(),
        ) * TILE;
        let rounded = normalize(data, rounded, half_view);
        let mut delta = rounded - display;
        let period = Vec2::new(data.width as f32, data.height as f32) * TILE;
        for axis in 0..2 {
            if [data.loops_x(), data.loops_y()][axis] {
                delta[axis] = (delta[axis] + period[axis] / 2.0).rem_euclid(period[axis])
                    - period[axis] / 2.0;
            }
        }
        let position = position + delta * Vec2::new(1.0, -1.0);
        state.world = position.to_array();
        state.display = rounded.to_array();
        self.position = Some(position);
    }
}

fn normalize(data: &MapData, mut display: Vec2, half_view: Vec2) -> Vec2 {
    let period = Vec2::new(data.width as f32, data.height as f32) * TILE;
    for axis in 0..2 {
        display[axis] = if [data.loops_x(), data.loops_y()][axis] {
            display[axis].rem_euclid(period[axis])
        } else {
            display[axis].clamp(0.0, (period[axis] - half_view[axis] * 2.0).max(0.0))
        };
    }
    display
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landing_preserves_panorama_phase_until_a_real_scroll_resets_its_axis() {
        let map = MapData::for_test(40, 30);
        let half = Vec2::new(160.0, 120.0);
        let mut pan = CameraPan::default();
        pan.update(&map, Vec2::new(8.0, -8.0), half, 0.0);
        pan.scroll_to(&map, Vec2::new(24.25, -11.5), half);
        let before = pan.panorama_position(&map, [false; 2]).unwrap();
        pan.round_jump(&map, half);
        assert_eq!(pan.position, Some(Vec2::new(32.0, -8.0)));
        assert_eq!(pan.effects_position(), Some(Vec2::new(24.25, -11.5)));
        assert_eq!(pan.panorama_position(&map, [false; 2]), Some(before));
        assert_eq!(pan.panorama_position(&map, [true; 2]), Some(before));
        pan.scroll_to(&map, Vec2::new(33.0, -8.0), half);
        assert_eq!(
            pan.panorama_position(&map, [false; 2]),
            Some(Vec2::new(193.0, before.y))
        );
        assert_eq!(
            pan.panorama_position(&map, [true; 2]),
            Some(before + Vec2::X)
        );
    }

    #[test]
    fn same_map_relocation_changes_the_view_without_scrolling_effects() {
        let map = MapData::for_test(40, 30);
        let half = Vec2::new(160.0, 120.0);
        let mut pan = CameraPan::default();
        pan.update(&map, Vec2::ZERO, half, 0.0);
        let before = pan.effects_position();
        pan.recenter(false);
        pan.update(&map, Vec2::new(80.0, -64.0), half, 0.0);
        assert_eq!(pan.position, Some(Vec2::new(88.0, -64.0)));
        assert_eq!(pan.effects_position(), before);
        pan.recenter(true);
        pan.update(&map, Vec2::new(-32.0, 32.0), half, 0.0);
        assert_eq!(pan.effects_position(), pan.position);
    }

    #[test]
    fn every_nonfinite_tracking_field_is_rejected_on_load() {
        let map = MapData::for_test(40, 30);
        let mut pan = CameraPan::default();
        pan.update(&map, Vec2::ZERO, Vec2::new(160.0, 120.0), 0.0);
        let original = pan.snapshot();
        assert!(original.valid());
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for field in 0..6 {
                for axis in 0..2 {
                    let mut saved = original.clone();
                    let tracking = saved.tracking.as_mut().unwrap();
                    match field {
                        0 => tracking.world[axis] = invalid,
                        1 => tracking.display[axis] = invalid,
                        2 => tracking.effects[axis] = invalid,
                        3 => tracking.panorama[axis] = invalid,
                        4 => tracking.bounded_panorama[axis] = invalid,
                        _ => tracking.initial_display[axis] = invalid,
                    }
                    assert!(!saved.valid());
                }
            }
        }
    }
}
