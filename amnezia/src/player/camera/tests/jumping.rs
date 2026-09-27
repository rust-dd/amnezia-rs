use super::*;
use crate::world::RouteStepper;

mod logical;
mod loops;
mod saved;

struct Reference {
    display: [i32; 2],
    pan: [i32; 2],
    destination: [i32; 2],
    delta: [i32; 2],
    remaining: i32,
    amount: i32,
}

impl Reference {
    fn update(&mut self) {
        for axis in 0..2 {
            let period = [40 * 256, 30 * 256][axis];
            let gap = self.destination[axis] - self.display[axis] - self.pan[axis];
            let gap = (gap + period / 2).rem_euclid(period) - period / 2;
            self.display[axis] = (self.display[axis]
                + self.amount * gap.signum() * self.delta[axis].abs())
            .clamp(0, period - [320 * 16, 240 * 16][axis]);
        }
        self.remaining = (self.remaining - self.amount).max(0);
        if self.remaining == 0 {
            for position in &mut self.display {
                *position = (f64::from(*position) / 256.0).round_ties_even() as i32 * 256;
            }
        }
        let step = (self.pan[0] - 8 * 256).clamp(-4, 4);
        let after = (self.display[0] + step).clamp(0, 20 * 256);
        self.pan[0] -= after - self.display[0];
        self.display[0] = after;
    }

    fn camera(&self) -> Vec2 {
        Vec2::new(
            self.display[0] as f32 / 16.0 - 160.0,
            120.0 - self.display[1] as f32 / 16.0,
        )
    }
}

fn run(tile: (i32, i32), speed: u32, delta: (i32, i32), pan_frames: i32) {
    let (mut app, hero, _) = super::walking::fixture(tile);
    app.world_mut()
        .resource_mut::<CameraPan>()
        .command(&[2, 1, 1, 1, 0]);
    for _ in 0..pan_frames {
        app.update();
    }
    let mut params = vec![10001, 8, 0, 0, 36, 24];
    params.extend(std::iter::repeat_n(
        if delta.0 > 0 { 1 } else { 3 },
        delta.0.unsigned_abs() as usize,
    ));
    params.extend(std::iter::repeat_n(
        if delta.1 > 0 { 2 } else { 0 },
        delta.1.unsigned_abs() as usize,
    ));
    params.extend([25, 37]);
    let mut route = app.world_mut().get_mut::<RouteStepper>(hero).unwrap();
    route.set_speed(speed);
    route.force_route(RouteStepper::from_move_event(&params));
    let mut reference = Reference {
        display: [(tile.0 - 9) * 256 + pan_frames * 4, (tile.1 - 7) * 256],
        pan: [9 * 256 - pan_frames * 4, 7 * 256],
        destination: [(tile.0 + delta.0) * 256, (tile.1 + delta.1) * 256],
        delta: [delta.0, delta.1],
        remaining: 256,
        amount: [8, 12, 16, 24, 32, 64][(speed - 1) as usize],
    };
    while reference.remaining > 0 {
        reference.update();
        app.update();
        assert_eq!(
            app.world().resource::<CameraPan>().position,
            Some(reference.camera()),
            "speed {speed}, delta {delta:?}, remaining {}",
            reference.remaining
        );
        assert_eq!(
            app.world().get::<MoveQueue>(hero).unwrap().busy(),
            reference.remaining > 0
        );
    }
}

#[test]
fn a_jump_uses_the_full_last_amount_and_rounds_the_camera_before_panning() {
    run((20, 15), 2, (3, 2), 33);
}

#[test]
fn a_jump_in_place_still_rounds_the_camera_at_landing() {
    run((20, 15), 6, (0, 0), 33);
}

#[test]
fn a_vertical_jump_also_rounds_the_stationary_camera_axis() {
    run((20, 15), 4, (0, 2), 33);
}

#[test]
fn landing_rounds_half_tiles_to_even_in_both_directions() {
    for x in [20, 21] {
        run((x, 15), 6, (0, 0), 29);
    }
}

#[test]
fn all_jump_speeds_and_directions_keep_the_original_scroll_sequence() {
    for speed in 1..=6 {
        for delta in [(0, 0), (3, 0), (-3, 0), (0, 2), (0, -2), (3, -2), (-3, 2)] {
            run((20, 15), speed, delta, 33);
        }
    }
}

#[test]
fn locking_suppresses_jump_scroll_and_landing_rounding_but_not_pan() {
    let (mut app, hero, origin) = super::walking::fixture((20, 15));
    let mut pan = app.world_mut().resource_mut::<CameraPan>();
    pan.command(&[2, 1, 1, 1, 0]);
    pan.locked = true;
    let mut route = app.world_mut().get_mut::<RouteStepper>(hero).unwrap();
    route.set_speed(6);
    route.force_route(RouteStepper::from_move_event(&[
        10001, 8, 0, 0, 24, 1, 1, 25,
    ]));
    for frame in 1..=4 {
        app.update();
        assert_eq!(
            app.world().resource::<CameraPan>().position,
            Some(origin + Vec2::X * (frame as f32 / 4.0))
        );
    }
    assert!(!app.world().get::<MoveQueue>(hero).unwrap().busy());
}
