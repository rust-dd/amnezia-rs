use super::*;
use crate::world::{Character, RouteStepper, ScrollStep};

#[derive(Resource, Default)]
pub(in crate::player) struct MotionScroll(Option<Frame>);

struct Frame {
    step: ScrollStep,
    direction: u32,
    half_view: Vec2,
}

pub(in crate::player) fn prepare_scroll(
    time: Res<Time>,
    data: Res<MapData>,
    scene: ScenePause,
    mut pan: ResMut<CameraPan>,
    mut frame: ResMut<MotionScroll>,
    mut heroes: Query<(&Player, &mut MoveQueue, &RouteStepper)>,
    cameras: Query<&Projection, With<MainCamera>>,
) {
    frame.0 = None;
    if scene.paused() || scene.riding() {
        return;
    }
    let Ok((hero, mut queue, route)) = heroes.single_mut() else {
        return;
    };
    let Ok(Projection::Orthographic(view)) = cameras.single() else {
        return;
    };
    let direction = route.direction(hero);
    queue.use_character_motion(route.speed(), direction);
    let Some(step) = queue.scroll_step(hero.tile(), time.delta_secs()) else {
        return;
    };
    let half_view = view.area.size() / 2.0;
    if pan.position.is_none() {
        pan.update(&data, queue.subpixel_position(hero, &data), half_view, 0.0);
    }
    frame.0 = Some(Frame {
        step,
        direction,
        half_view,
    });
}

pub(in crate::player) fn apply_scroll(
    data: Res<MapData>,
    mut pan: ResMut<CameraPan>,
    mut frame: ResMut<MotionScroll>,
    heroes: Query<(&Player, &MoveQueue)>,
) {
    let Some(frame) = frame.0.take() else {
        return;
    };
    let Ok((hero, queue)) = heroes.single() else {
        return;
    };
    if !pan.locked {
        let position = pan.position.unwrap();
        let focus =
            Vec2::from(data.tile_center(hero.tile_x, hero.tile_y)) + Vec2::X * 8.0 + pan.offset;
        let period = Vec2::new(data.width as f32, data.height as f32) * TILE;
        let mut gap = (focus - position) * Vec2::new(1.0, -1.0);
        let (dx, dy) = crate::world::dir_delta(frame.direction);
        let direction = Vec2::new(dx as f32, dy as f32);
        let mut moved = Vec2::ZERO;
        for axis in 0..2 {
            gap[axis] =
                (gap[axis] + period[axis] / 2.0).rem_euclid(period[axis]) - period[axis] / 2.0;
            let sign = if gap[axis] == 0.0 {
                0.0
            } else {
                gap[axis].signum()
            };
            if let Some(delta) = frame.step.jump_delta {
                moved[axis] = sign * delta[axis] * frame.step.pixels;
            } else if gap[axis] * direction[axis] > 0.0 {
                moved[axis] = sign * frame.step.pixels;
            }
        }
        let position = clamp_position(
            &data,
            position + moved * Vec2::new(1.0, -1.0),
            frame.half_view,
        );
        pan.scroll_to(&data, position, frame.half_view);
        if frame.step.landing {
            pan.round_jump(&data, frame.half_view);
        }
    }
    let point = queue.subpixel_position(hero, &data);
    pan.previous_player = Some(data.world_near(point, pan.previous_player.unwrap_or(point)));
}
