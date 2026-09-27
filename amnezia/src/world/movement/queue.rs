use super::{Character, MapData, RouteAction, STEP_DURATION, center, dir_delta};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Kinematics {
    pub speed: u32,
    pub direction: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Subpixels {
    pub remaining: u16,
    pub fraction: f64,
}

impl Subpixels {
    fn advance(&mut self, speed: u32, jumping: bool, dt: f32) {
        let frames = self.fraction + f64::from(dt.max(0.0)) * 60.0;
        let whole = (frames + 0.000001).floor();
        self.fraction = (frames - whole).max(0.0);
        let amount = if jumping {
            [8_u32, 12, 16, 24, 32, 64][(speed - 1) as usize]
        } else {
            1 << (1 + speed)
        };
        self.remaining =
            u32::from(self.remaining).saturating_sub(amount.saturating_mul(whole as u32)) as u16;
    }

    fn height(self) -> f32 {
        let height = self.remaining.min(256 - self.remaining) / 8;
        if height < 5 {
            f32::from(height * 2)
        } else if height < 13 {
            f32::from(height + 4)
        } else {
            16.0
        }
    }
}

#[derive(Clone)]
pub(super) struct Tween {
    pub from: Vec2,
    pub to: Vec2,
    pub elapsed: f32,
    pub jumping: bool,
    pub subpixels: Option<Subpixels>,
}

/// Pending moves and the original character's remaining-step counter.
/// Animation and stop clocks belong to the route, not to individual moves.
#[derive(Component, Clone)]
pub struct MoveQueue {
    pub(super) steps: VecDeque<RouteAction>,
    pub(super) active: Option<Tween>,
    pub(super) step_secs: f32,
    pub(super) jump_attempt: bool,
    pub(super) kinematics: Option<Kinematics>,
}

impl Default for MoveQueue {
    fn default() -> Self {
        Self {
            steps: default(),
            active: None,
            step_secs: STEP_DURATION,
            jump_attempt: false,
            kinematics: None,
        }
    }
}

impl MoveQueue {
    pub fn enqueue_route(&mut self, actions: impl IntoIterator<Item = RouteAction>) {
        self.steps.extend(actions);
    }

    pub fn push_step(&mut self, action: RouteAction) {
        self.steps.push_back(action);
    }

    /// Duration used by legacy snapshots and detached movement queues.
    pub fn set_step_secs(&mut self, secs: f32) {
        self.step_secs = secs;
    }

    pub(crate) fn use_character_motion(&mut self, speed: u32, direction: u32) {
        self.kinematics = Some(Kinematics {
            speed: speed.clamp(1, 6),
            direction,
        });
        if let Some(tween) = &mut self.active {
            tween.subpixels.get_or_insert_with(|| Subpixels {
                remaining: (256.0 * (1.0 - tween.elapsed / self.step_secs))
                    .round()
                    .clamp(1.0, 256.0) as u16,
                fraction: 0.0,
            });
        }
    }

    pub fn busy(&self) -> bool {
        self.jump_attempt || self.active.is_some() || !self.steps.is_empty()
    }

    pub(crate) fn walking_scroll_pixels(&self, dt: f32) -> Option<f32> {
        let motion = self.kinematics?;
        let counter = if let Some(tween) = &self.active {
            if tween.jumping {
                return None;
            }
            tween.subpixels?
        } else if matches!(self.steps.front(), Some(RouteAction::Step { .. })) {
            Subpixels {
                remaining: 256,
                fraction: 0.0,
            }
        } else {
            return None;
        };
        let amount = 1_u32 << (1 + motion.speed);
        let frames = (counter.fraction + f64::from(dt.max(0.0)) * 60.0 + 0.000001).floor();
        let ticks = (frames as u32).min(u32::from(counter.remaining).div_ceil(amount));
        Some((amount * ticks) as f32 / 16.0)
    }

    pub(crate) fn render_position<C: Character>(&self, ch: &C, data: &MapData) -> Vec2 {
        let mut point = self.ground_position(ch, data);
        if let Some(tween) = &self.active
            && tween.jumping
        {
            point.y += tween.subpixels.map_or_else(
                || {
                    let remaining =
                        (256.0 * (1.0 - tween.elapsed / self.step_secs)).clamp(0.0, 256.0) as u16;
                    Subpixels {
                        remaining,
                        fraction: 0.0,
                    }
                    .height()
                },
                Subpixels::height,
            );
        }
        point
    }

    pub(crate) fn ground_position<C: Character>(&self, ch: &C, data: &MapData) -> Vec2 {
        let point = self.subpixel_position(ch, data);
        let Some(tween) = &self.active else {
            return point;
        };
        if tween.subpixels.is_none() {
            return point;
        }
        // RPG_RT divides canonical map coordinates before wrapping screen positions.
        let shift = center(data, ch.tile().0, ch.tile().1) - tween.to;
        let raw = point + shift;
        Vec2::new(
            (raw.x + data.offset_x - 8.0).trunc() + 8.0 - data.offset_x,
            data.offset_y - (data.offset_y - raw.y - 8.0).trunc() - 8.0,
        ) - shift
    }

    pub(crate) fn subpixel_position<C: Character>(&self, ch: &C, data: &MapData) -> Vec2 {
        let Some(tween) = &self.active else {
            return center(data, ch.tile().0, ch.tile().1);
        };
        if let Some(subpixels) = tween.subpixels {
            let remaining = f32::from(subpixels.remaining) / 256.0;
            if !tween.jumping
                && let Some(kinematics) = self.kinematics
            {
                let (dx, dy) = dir_delta(kinematics.direction);
                return tween.to - Vec2::new(dx as f32, -dy as f32) * (16.0 * remaining);
            }
            tween.from.lerp(tween.to, 1.0 - remaining)
        } else {
            tween
                .from
                .lerp(tween.to, (tween.elapsed / self.step_secs).clamp(0.0, 1.0))
        }
    }

    pub(crate) fn jumping(&self) -> bool {
        self.jump_attempt || self.active.as_ref().is_some_and(|tween| tween.jumping)
    }

    pub(in crate::world) fn set_jump_attempt(&mut self, jumping: bool) {
        self.jump_attempt = jumping;
    }

    /// Advances at most one move; its successor belongs to the next character update.
    pub(crate) fn advance<C: Character>(
        &mut self,
        ch: &mut C,
        data: &MapData,
        dt: f32,
    ) -> Option<Vec2> {
        if self.active.is_none() {
            let action = self.steps.pop_front()?;
            self.begin_from(ch, data, ch.tile(), action);
        }
        let tween = self.active.as_mut().unwrap();
        let finished =
            if let (Some(subpixels), Some(kinematics)) = (&mut tween.subpixels, self.kinematics) {
                subpixels.advance(kinematics.speed, tween.jumping, dt);
                tween.elapsed = self.step_secs * (1.0 - f32::from(subpixels.remaining) / 256.0);
                subpixels.remaining == 0
            } else {
                tween.elapsed += dt;
                tween.elapsed >= self.step_secs
            };
        if finished {
            let end = tween.to;
            self.active = None;
            Some(end)
        } else {
            Some(self.render_position(ch, data))
        }
    }

    pub(crate) fn begin_from<C: Character>(
        &mut self,
        ch: &mut C,
        data: &MapData,
        origin: (i32, i32),
        action: RouteAction,
    ) {
        let (dx, dy) = action.delta();
        let jumping = matches!(action, RouteAction::Jump { .. });
        let (RouteAction::Step { face, .. } | RouteAction::Jump { face, .. }) = action;
        let (x, y) = origin;
        let (nx, ny) = (x + dx, y + dy);
        let (tile_x, tile_y) = data.normalize_tile(nx, ny);
        ch.set_tile(tile_x, tile_y);
        ch.set_dir(face);
        self.active = Some(Tween {
            from: center(data, x, y),
            to: center(data, nx, ny),
            elapsed: 0.0,
            jumping,
            subpixels: self.kinematics.map(|_| Subpixels {
                remaining: 256,
                fraction: 0.0,
            }),
        });
    }
}
