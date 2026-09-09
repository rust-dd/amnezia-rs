//! Unit tests for the [`super::RouteStepper`] state machine: one command
//! kind per test — a move steps in the faced direction (skipped when blocked and
//! skippable, waited on otherwise), a face turns in place, a wait arms the delay,
//! and a route repeats or stops at its end — plus the `MoveEvent` route decode.

use super::*;

/// A minimal test character: tracks tile, facing, frame, and graphic.
struct TestChar {
    x: i32,
    y: i32,
    dir: u32,
    frame: u32,
    charset: String,
    index: u32,
}

impl TestChar {
    fn new() -> Self {
        Self {
            x: 5,
            y: 5,
            dir: DIR_DOWN,
            frame: 1,
            charset: "C".into(),
            index: 0,
        }
    }
}

impl Character for TestChar {
    fn tile(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    fn set_tile(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
    fn dir(&self) -> u32 {
        self.dir
    }
    fn set_dir(&mut self, dir: u32) {
        self.dir = dir;
    }
    fn frame(&self) -> u32 {
        self.frame
    }
    fn set_frame(&mut self, frame: u32) {
        self.frame = frame;
    }
    fn index(&self) -> u32 {
        self.index
    }
    fn charset(&self) -> &str {
        &self.charset
    }
    fn set_graphic(&mut self, name: String, index: u32) {
        self.charset = name;
        self.index = index;
    }
}

fn stepper(codes: &[u32], repeat: bool, skippable: bool) -> RouteStepper {
    let commands = codes
        .iter()
        .map(|&code| MoveCommandDef {
            code,
            params: Vec::new(),
            string: String::new(),
        })
        .collect();
    RouteStepper::new(commands, repeat, skippable, 4, 8, false)
}

fn open(_dx: i32, _dy: i32, _jumping: bool) -> bool {
    true
}
fn blocked(_dx: i32, _dy: i32, _jumping: bool) -> bool {
    false
}

#[test]
fn move_command_steps_in_the_faced_direction() {
    let mut ch = TestChar::new();
    let mut s = stepper(&[3], false, false); // move-left
    let mut fx = Vec::new();
    let action = s.advance(&mut ch, (0, 0), &open, &mut fx);
    assert!(matches!(
        action,
        Some((
            RouteAction::Step {
                dx: -1,
                dy: 0,
                face: DIR_LEFT
            },
            _
        ))
    ));
    assert_eq!(ch.dir(), DIR_LEFT);
    assert_eq!(s.index, 1);
}

#[test]
fn blocked_skippable_move_skips_to_next_command() {
    // move-left (blocked) then face-up: the skippable route skips the move
    // (index past it, facing restored) and runs the face.
    let mut ch = TestChar::new();
    let mut s = stepper(&[3, 12], false, true);
    let mut fx = Vec::new();
    let action = s.advance(&mut ch, (0, 0), &blocked, &mut fx);
    assert!(action.is_none()); // ended on the face (a turn gates without a step)
    assert_eq!(ch.dir(), DIR_UP);
    assert_eq!(s.index, 2);
}

#[test]
fn blocked_nonskippable_move_waits_on_the_command() {
    // A non-skippable blocked move faces the obstacle and stays on the command.
    let mut ch = TestChar::new();
    let mut s = stepper(&[3], false, false);
    s.frequency = 3; // freq 8 has a zero step delay; use 3 for a real countdown
    let mut fx = Vec::new();
    let action = s.advance(&mut ch, (0, 0), &blocked, &mut fx);
    assert!(action.is_none());
    assert_eq!(ch.dir(), DIR_LEFT); // faced the wall
    assert_eq!(s.index, 0); // did not advance — retries next tick
    assert!(s.timer > 0.0);
}

#[test]
fn face_command_turns_without_moving() {
    let mut ch = TestChar::new();
    let mut s = stepper(&[12], false, false); // face-up
    let mut fx = Vec::new();
    let action = s.advance(&mut ch, (0, 0), &open, &mut fx);
    assert!(action.is_none());
    assert_eq!(ch.dir(), DIR_UP);
    assert_eq!(s.index, 1);
}

#[test]
fn wait_command_arms_the_delay() {
    let mut ch = TestChar::new();
    let mut s = stepper(&[23], false, false);
    s.frequency = 3; // freq 8 has a zero wait; use 3 for a real countdown
    let mut fx = Vec::new();
    let action = s.advance(&mut ch, (0, 0), &open, &mut fx);
    assert!(action.is_none());
    assert!(s.timer > 0.0);
    assert_eq!(s.index, 1);
}

#[test]
fn repeat_route_loops_back_to_the_start() {
    let mut ch = TestChar::new();
    let mut s = stepper(&[2], true, false); // move-down, repeating
    let mut fx = Vec::new();
    assert!(s.advance(&mut ch, (0, 0), &open, &mut fx).is_some());
    assert_eq!(s.index, 1);
    // Still active and wraps: the next advance re-runs the one command.
    assert!(s.active());
    assert!(s.advance(&mut ch, (0, 0), &open, &mut fx).is_some());
    assert_eq!(s.index, 1);
}

#[test]
fn oneshot_route_stops_at_the_end() {
    let mut ch = TestChar::new();
    let mut s = stepper(&[2], false, false);
    let mut fx = Vec::new();
    assert!(s.advance(&mut ch, (0, 0), &open, &mut fx).is_some());
    // Draining the last command deactivates the stepper.
    assert!(s.advance(&mut ch, (0, 0), &open, &mut fx).is_none());
    assert!(!s.active());
}

#[test]
fn instant_commands_apply_then_reach_the_gate() {
    // switch-on (32), change-graphic (34) "Ron"/2, then move-down: the two
    // instant commands apply and the move gates, all in one advance.
    let mut ch = TestChar::new();
    let commands = vec![
        MoveCommandDef {
            code: 32,
            params: vec![7],
            string: String::new(),
        },
        MoveCommandDef {
            code: 34,
            params: vec![2],
            string: "Ron".into(),
        },
        MoveCommandDef {
            code: 2,
            params: Vec::new(),
            string: String::new(),
        },
    ];
    let mut s = RouteStepper::new(commands, false, false, 4, 8, false);
    let mut fx = Vec::new();
    let action = s.advance(&mut ch, (0, 0), &open, &mut fx);
    assert!(matches!(action, Some((RouteAction::Step { dy: 1, .. }, _))));
    assert_eq!(ch.charset(), "Ron");
    assert_eq!(ch.index(), 2);
    assert!(matches!(fx.as_slice(), [StepEffect::Switch(7, true)]));
    assert_eq!(s.index, 3);
}

#[test]
fn through_ignores_a_block() {
    // walk-everywhere-on (36) then a blocked move still steps.
    let mut ch = TestChar::new();
    let mut s = stepper(&[36, 3], false, false);
    let mut fx = Vec::new();
    let action = s.advance(&mut ch, (0, 0), &blocked, &mut fx);
    assert!(matches!(
        action,
        Some((RouteAction::Step { dx: -1, .. }, _))
    ));
}

#[test]
fn decode_move_event_reads_flags_and_commands() {
    // [hero, freq 8, repeat 1, skip 0, move-left, face-up]
    let s = RouteStepper::from_move_event(&[10001, 8, 1, 0, 3, 12]);
    assert!(s.repeat && !s.skippable);
    assert_eq!(s.frequency, 8);
    assert_eq!(s.commands.len(), 2);
    assert_eq!(s.commands[0].code, 3);
    assert_eq!(s.commands[1].code, 12);
}

#[test]
fn decode_move_event_reads_change_graphic_string() {
    // change_graphic "Torch" (len 5, one byte per int) frame 1, then face-left.
    let params = vec![10005, 8, 0, 0, 34, 5, 84, 111, 114, 99, 104, 1, 15];
    let s = RouteStepper::from_move_event(&params);
    assert_eq!(s.commands.len(), 2);
    assert_eq!(s.commands[0].code, 34);
    assert_eq!(s.commands[0].string, "Torch");
    assert_eq!(s.commands[0].params, vec![1]);
    assert_eq!(s.commands[1].code, 15);
}

#[test]
fn toward_and_away_pick_opposite_directions() {
    // Hero three tiles right of the NPC at (5,5): toward = right, away = left.
    assert_eq!(toward_dir((8, 5), (5, 5)), DIR_RIGHT);
    assert_eq!(away_dir((8, 5), (5, 5)), DIR_LEFT);
}
