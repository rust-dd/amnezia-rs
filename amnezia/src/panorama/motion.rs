use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Motion {
    pub(super) name: Option<String>,
    pub(super) size: [u32; 2],
    pub(super) phase: [i64; 2],
    on_map_init: bool,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            name: None,
            size: [0; 2],
            phase: [0; 2],
            on_map_init: true,
        }
    }
}

impl Motion {
    pub(super) fn valid(&self) -> bool {
        self.size.iter().all(|&size| size <= i32::MAX as u32 / 32)
            && self.phase.iter().enumerate().all(|(axis, phase)| {
                let size = if self.size[axis] == 0 {
                    i32::MAX as u32 / 32
                } else {
                    self.size[axis]
                };
                phase.unsigned_abs() < u64::from(size) * 32
            })
    }

    pub(super) fn initialize(
        &mut self,
        name: &str,
        size: UVec2,
        definition: Option<&PanoramaDef>,
        map: &MapData,
        display: Vec2,
    ) {
        if self.name.as_deref() == Some(name) && self.size == size.to_array() {
            return;
        }
        self.size = size.to_array();
        if self.on_map_init {
            for axis in 0..2 {
                self.set(axis, subpixels(display[axis]));
            }
        }
        if let Some(definition) = definition {
            self.reset_bounded(definition, map, display, [true; 2]);
        }
        for axis in 0..2 {
            self.set(axis, self.phase[axis]);
        }
        self.name = Some(name.into());
        self.on_map_init = false;
    }

    pub(super) fn scroll(
        &mut self,
        event: &crate::player::BackgroundScroll,
        definition: Option<&PanoramaDef>,
        map: &MapData,
    ) {
        if event.map_id != map.map_id {
            return;
        }
        let display = Vec2::from_array(event.display);
        if let Some(delta) = event.delta {
            let Some(definition) = definition else { return };
            for axis in 0..2 {
                if delta[axis] != 0.0 && [definition.loop_x, definition.loop_y][axis] {
                    self.set(axis, self.phase[axis] + subpixels(delta[axis]));
                }
            }
            self.reset_bounded(definition, map, display, delta.map(|value| value != 0.0));
        } else {
            for axis in 0..2 {
                self.set(axis, subpixels(display[axis]));
            }
            if let Some(definition) = definition {
                self.reset_bounded(definition, map, display, [true; 2]);
            }
        }
    }

    fn reset_bounded(
        &mut self,
        definition: &PanoramaDef,
        map: &MapData,
        display: Vec2,
        reset: [bool; 2],
    ) {
        for axis in 0..2 {
            if !reset[axis]
                || [definition.loop_x, definition.loop_y][axis]
                || [map.loops_x(), map.loops_y()][axis]
            {
                continue;
            }
            let viewport = [320, 240][axis];
            let span = i64::from([map.width, map.height][axis]) * 16 - viewport;
            let image_span = i64::from(self.size[axis]) - viewport;
            if span > 0 && image_span > 0 {
                let phase = 2 * span.min(image_span) * subpixels(display[axis]) / span;
                self.set(axis, phase);
            } else {
                self.phase[axis] = 0;
            }
        }
    }

    fn set(&mut self, axis: usize, value: i64) {
        let period = i64::from(self.size[axis]) * 32;
        if period != 0 {
            self.phase[axis] = (value + period) % period;
        }
    }

    pub(super) fn step(&mut self, definition: &PanoramaDef, frames: u32) {
        for _ in 0..frames {
            for axis in 0..2 {
                if [definition.loop_x, definition.loop_y][axis]
                    && [definition.auto_x, definition.auto_y][axis]
                    && [definition.speed_x, definition.speed_y][axis] != 0
                {
                    self.set(
                        axis,
                        self.phase[axis] + amount([definition.speed_x, definition.speed_y][axis]),
                    );
                }
            }
        }
    }

    pub(super) fn offset(&self) -> Vec2 {
        Vec2::from_array(std::array::from_fn(|axis| {
            let offset = (-self.phase[axis] / 16) / 2;
            offset.rem_euclid(i64::from(self.size[axis].max(1))) as f32
        }))
    }

    pub(super) fn migrate(
        definition: &PanoramaDef,
        map: &MapData,
        size: UVec2,
        display: Vec2,
        scroll: (f32, f32),
    ) -> Self {
        let mut motion = Self {
            name: Some(definition.name.clone()),
            size: size.to_array(),
            on_map_init: false,
            ..default()
        };
        for axis in 0..2 {
            let span = [map.width, map.height][axis] as f32 * 16.0;
            let view = [320.0, 240.0][axis];
            let offset = camera_scroll(
                display[axis] - (span - view) / 2.0,
                span,
                view,
                size[axis] as f32,
                [
                    definition.loop_x || map.loops_x(),
                    definition.loop_y || map.loops_y(),
                ][axis],
            ) + [scroll.0, scroll.1][axis];
            let phase = (-f64::from(offset) * 32.0).rem_euclid(f64::from(size[axis]) * 32.0);
            motion.set(axis, phase.round() as i64);
        }
        motion
    }
}

fn subpixels(value: f32) -> i64 {
    (f64::from(value) * 16.0)
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i64
}

pub(super) fn amount(speed: i32) -> i64 {
    -i64::from(speed.signum()) * (1 << speed.saturating_abs().min(12))
}
