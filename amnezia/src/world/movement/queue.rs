use super::{Character, MapData, RouteAction, STEP_DURATION, center, dir_delta};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Kinematics {
    pub speed: u32,
    pub direction: u32,
}

impl Kinematics {
    fn amount(self, jumping: bool) -> u32 {
        if jumping {
            [8, 12, 16, 24, 32, 64][(self.speed - 1) as usize]
        } else {
            1 << (1 + self.speed)
        }
    }
}

pub(crate) struct ScrollStep {
    pub pixels: f32,
    pub jump_delta: Option<Vec2>,
    pub landing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Subpixels {
    pub remaining: u16,
    pub fraction: f64,
}

impl Subpixels {
    fn advance(&mut self, motion: Kinematics, jumping: bool, dt: f32) {
        let frames = self.fraction + f64::from(dt.max(0.0)) * 60.0;
        let whole = (frames + 0.000001).floor();
        self.fraction = (frames - whole).max(0.0);
        let amount = motion.amount(jumping);
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
    pub jump_origin: Option<(i32, i32)>,
}

impl Tween {
    fn finished(&self, seconds: f32) -> bool {
        self.subpixels
            .map_or(self.elapsed >= seconds, |clock| clock.remaining == 0)
    }

    fn jump_delta(&self, tile: (i32, i32)) -> Vec2 {
        self.jump_origin
            .map_or((self.to - self.from) / 16.0, |origin| {
                Vec2::new(
                    tile.0 as f32 - origin.0 as f32,
                    tile.1 as f32 - origin.1 as f32,
                )
            })
            .abs()
    }
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

    #[cfg(test)]
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
                    .clamp(if tween.jumping { 0.0 } else { 1.0 }, 256.0)
                    as u16,
                fraction: 0.0,
            });
        }
    }

    pub fn busy(&self) -> bool {
        self.jump_attempt || self.active.is_some() || !self.steps.is_empty()
    }

    pub(crate) fn scroll_step(&self, tile: (i32, i32), dt: f32) -> Option<ScrollStep> {
        let motion = self.kinematics?;
        let (counter, jump_delta) = if let Some(tween) = &self.active {
            (
                tween.subpixels?,
                tween.jumping.then(|| tween.jump_delta(tile)),
            )
        } else {
            let action = self.steps.front()?;
            let (dx, dy) = action.delta();
            (
                Subpixels {
                    remaining: 256,
                    fraction: 0.0,
                },
                matches!(action, RouteAction::Jump { .. })
                    .then_some(Vec2::new(dx as f32, dy as f32).abs()),
            )
        };
        let amount = motion.amount(jump_delta.is_some());
        let frames = (counter.fraction + f64::from(dt.max(0.0)) * 60.0 + 0.000001).floor();
        let ticks = (frames as u32).min(u32::from(counter.remaining).div_ceil(amount).max(1));
        Some(ScrollStep {
            pixels: (amount * ticks) as f32 / 16.0,
            jump_delta,
            landing: jump_delta.is_some() && amount * ticks >= u32::from(counter.remaining),
        })
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
        if tween.subpixels.is_none() || tween.finished(self.step_secs) {
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
        if tween.finished(self.step_secs) {
            return center(data, ch.tile().0, ch.tile().1);
        }
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

    pub(crate) fn set_jump_attempt(&mut self, jumping: bool) {
        self.jump_attempt = jumping;
    }

    pub(crate) fn relocate(&mut self, previous_tile: (i32, i32)) {
        self.steps.clear();
        if let Some(tween) = &mut self.active
            && tween.jumping
        {
            tween.jump_origin.get_or_insert_with(|| {
                let delta = (tween.to - tween.from) / 16.0;
                (
                    previous_tile.0.saturating_sub(delta.x as i32),
                    previous_tile.1.saturating_add(delta.y as i32),
                )
            });
            tween.elapsed = self.step_secs;
            if let Some(clock) = &mut tween.subpixels {
                clock.remaining = 0;
            }
        } else {
            self.active = None;
        }
    }

    pub(crate) fn sync_remaining<C: Character>(
        &mut self,
        rider: &Self,
        previous_tile: (i32, i32),
        character: &C,
        data: &MapData,
    ) {
        let remaining = rider.active.as_ref().map_or(0, |tween| {
            tween.subpixels.map_or_else(
                || {
                    (256.0 * (1.0 - tween.elapsed / rider.step_secs))
                        .round()
                        .clamp(0.0, 256.0) as u16
                },
                |clock| clock.remaining,
            )
        });
        self.steps.clear();
        let previous = self.active.take();
        let jumping = previous.as_ref().is_some_and(|tween| tween.jumping);
        if remaining == 0 && !jumping {
            return;
        }
        let jump_origin = previous.as_ref().filter(|_| jumping).map(|tween| {
            tween.jump_origin.unwrap_or_else(|| {
                let delta = (tween.to - tween.from) / 16.0;
                (
                    previous_tile.0.saturating_sub(delta.x as i32),
                    previous_tile.1.saturating_add(delta.y as i32),
                )
            })
        });
        let to = center(data, character.tile().0, character.tile().1);
        let fraction = previous
            .and_then(|tween| tween.subpixels)
            .map_or(0.0, |clock| clock.fraction);
        self.active = Some(Tween {
            from: jump_origin.map_or(to, |origin| center(data, origin.0, origin.1)),
            to,
            elapsed: self.step_secs * (1.0 - f32::from(remaining) / 256.0),
            jumping,
            subpixels: Some(Subpixels {
                remaining,
                fraction,
            }),
            jump_origin,
        });
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
        let relocated = tween.finished(self.step_secs);
        let finished =
            if let (Some(subpixels), Some(kinematics)) = (&mut tween.subpixels, self.kinematics) {
                subpixels.advance(kinematics, tween.jumping, dt);
                tween.elapsed = self.step_secs * (1.0 - f32::from(subpixels.remaining) / 256.0);
                subpixels.remaining == 0
            } else {
                tween.elapsed += dt;
                tween.elapsed >= self.step_secs
            };
        if finished {
            let end = if relocated {
                center(data, ch.tile().0, ch.tile().1)
            } else {
                tween.to
            };
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
            jump_origin: jumping.then_some((tile_x - dx, tile_y - dy)),
            subpixels: self.kinematics.map(|_| Subpixels {
                remaining: 256,
                fraction: 0.0,
            }),
        });
    }
}
