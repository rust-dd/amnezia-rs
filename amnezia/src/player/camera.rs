use super::{Player, ScenePause};
use crate::tiles::{CHAR_Y_OFFSET, TILE};
use crate::world::{MainCamera, MapData, MoveQueue};
use bevy::prelude::*;

pub(crate) mod saved;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CameraFollow;

#[derive(Resource)]
pub struct CameraPan {
    pub offset: Vec2,
    pub target: Vec2,
    pub speed: f32,
    pub locked: bool,
    pub(crate) position: Option<Vec2>,
    previous_player: Option<Vec2>,
}

impl Default for CameraPan {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            target: Vec2::ZERO,
            speed: 60.0,
            locked: false,
            position: None,
            previous_player: None,
        }
    }
}

impl CameraPan {
    pub(crate) fn command(&mut self, params: &[i32]) -> f32 {
        let op = params.first().copied().unwrap_or(0);
        match op {
            0 => self.locked = true,
            1 => self.locked = false,
            2 | 3 => {
                self.speed =
                    (2u32 << params.get(3).copied().unwrap_or(4).clamp(1, 6)) as f32 / TILE * 60.0;
                if op == 3 {
                    self.target = Vec2::ZERO;
                } else {
                    let distance = params.get(2).copied().unwrap_or(0) as f32 * TILE;
                    self.target += match params.get(1).copied().unwrap_or(0) {
                        0 => Vec2::Y * distance,
                        1 => Vec2::X * distance,
                        2 => Vec2::NEG_Y * distance,
                        3 => Vec2::NEG_X * distance,
                        _ => Vec2::ZERO,
                    };
                }
                if params.get(4).copied().unwrap_or(0) != 0 {
                    return ((self.target - self.offset).abs().max_element() / (self.speed / 60.0))
                        .ceil()
                        / 60.0;
                }
            }
            _ => {}
        }
        0.0
    }

    pub(crate) fn recenter(&mut self, map_changed: bool) {
        self.position = None;
        self.previous_player = None;
        if map_changed {
            self.offset = Vec2::ZERO;
            self.target = Vec2::ZERO;
            self.speed = 60.0;
        }
    }

    fn update(&mut self, data: &MapData, player: Vec2, half_view: Vec2, dt: f32) -> Vec2 {
        let player = data.world_near(player, self.previous_player.unwrap_or(player));
        let focus = player + Vec2::X * 8.0 + self.offset;
        let mut position = self
            .position
            .unwrap_or_else(|| clamp_position(data, focus, half_view));
        if !self.locked
            && let Some(previous) = self.previous_player
        {
            let moved = player - previous;
            let gap = focus - position;
            for axis in 0..2 {
                if moved[axis] * gap[axis] > 0.0 {
                    position[axis] += moved[axis].signum() * moved[axis].abs().min(gap[axis].abs());
                }
            }
            position = clamp_position(data, position, half_view);
        }
        let step = ease_toward(self.offset, self.target, self.speed * dt) - self.offset;
        let panned = clamp_position(data, position + step, half_view);
        self.offset += panned - position;
        self.position = Some(panned);
        self.previous_player = Some(player);
        panned
    }
}

fn clamp_position(data: &MapData, position: Vec2, half_view: Vec2) -> Vec2 {
    Vec2::new(
        if data.loops_x() {
            position.x
        } else {
            clamp_to_map(position.x, data.width as f32 * TILE / 2.0, half_view.x)
        },
        if data.loops_y() {
            position.y
        } else {
            clamp_to_map(position.y, data.height as f32 * TILE / 2.0, half_view.y)
        },
    )
}

pub(super) fn clamp_to_map(target: f32, half_map: f32, half_view: f32) -> f32 {
    if half_view >= half_map {
        0.0
    } else {
        target.clamp(-half_map + half_view, half_map - half_view)
    }
}

pub(super) fn ease_toward(offset: Vec2, target: Vec2, step: f32) -> Vec2 {
    offset + (target - offset).clamp(Vec2::splat(-step), Vec2::splat(step))
}

#[allow(clippy::type_complexity)]
pub(super) fn camera_follow(
    time: Res<Time>,
    data: Res<MapData>,
    scene: ScenePause,
    mut pan: ResMut<CameraPan>,
    players: Query<(&Player, &Transform, Option<&MoveQueue>)>,
    mut cameras: Query<(&mut Transform, &Projection), (With<MainCamera>, Without<Player>)>,
) {
    let Ok((player, transform, queue)) = players.single() else {
        return;
    };
    let Ok((mut camera, projection)) = cameras.single_mut() else {
        return;
    };
    let Projection::Orthographic(view) = projection else {
        return;
    };
    let mut point = transform.translation.truncate() - Vec2::Y * CHAR_Y_OFFSET;
    if !scene.riding()
        && let Some(queue) = queue
    {
        point = queue.subpixel_position(player, &data);
    }
    let half_view = view.area.size() / 2.0;
    let point = if scene.paused()
        && let Some(position) = pan.position
    {
        position
    } else {
        pan.update(
            &data,
            point,
            half_view,
            if scene.paused() {
                0.0
            } else {
                time.delta_secs()
            },
        )
    };
    let rendered = raster_position(&data, point, half_view);
    camera.translation.x = rendered.x;
    camera.translation.y = rendered.y;
}

fn raster_position(data: &MapData, point: Vec2, half_view: Vec2) -> Vec2 {
    let corner = Vec2::from(data.tile_center(0, 0)) + Vec2::new(-8.0, 8.0);
    Vec2::new(
        (point.x - corner.x - half_view.x).trunc() + corner.x + half_view.x,
        corner.y - half_view.y - (corner.y - point.y - half_view.y).trunc(),
    )
}

#[cfg(test)]
mod tests;
