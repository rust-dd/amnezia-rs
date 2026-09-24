use super::super::commands::{KeyAccept, decode_key_accept, key_code};
use super::super::frame::Frame;
use super::{Exec, Flow};
use amnezia_data::EventCommand;
use bevy::prelude::*;

pub(super) fn execute(frame: &mut Frame, x: &mut Exec, command: &EventCommand) -> Flow {
    let variable = command.params.first().copied().unwrap_or(0).max(0) as u32;
    let wait = command.params.get(1).copied().unwrap_or(0) != 0;
    let accept = decode_key_accept(&command.params);
    if wait {
        x.variables.set(variable, 0);
        if x.message_reserved() {
            return Flow::Yield;
        }
        frame.key_var = variable;
        frame.key_accept = accept;
        frame.key_pending = true;
        let keys = &mut x.subsystems.flow.keys;
        let triggered = keys.get_just_pressed().copied().collect::<Vec<_>>();
        // RPG_RT discards triggers here, but preserves held and released states.
        for key in triggered {
            keys.clear_just_pressed(key);
        }
        Flow::Yield
    } else {
        let code = sample(&accept, &x.subsystems.flow.keys, false);
        x.variables.set(variable, code);
        frame.ip += 1;
        Flow::Advance
    }
}

pub(super) fn resume(frame: &mut Frame, x: &mut Exec) -> bool {
    if !frame.key_pending {
        return true;
    }
    if x.message_active() {
        return false;
    }
    let code = sample(&frame.key_accept, &x.subsystems.flow.keys, true);
    x.variables.set(frame.key_var, code);
    if code == 0 {
        return false;
    }
    frame.key_pending = false;
    frame.ip += 1;
    true
}

fn sample(accept: &KeyAccept, keys: &ButtonInput<KeyCode>, wait: bool) -> i32 {
    let pressed = |key| {
        if wait {
            keys.just_pressed(key)
        } else {
            keys.pressed(key)
        }
    };
    key_code(
        accept,
        pressed(KeyCode::ArrowUp),
        pressed(KeyCode::ArrowDown),
        pressed(KeyCode::ArrowLeft),
        pressed(KeyCode::ArrowRight),
        pressed(KeyCode::Enter) || pressed(KeyCode::Space),
        pressed(KeyCode::Escape),
        pressed(KeyCode::ShiftLeft) || pressed(KeyCode::ShiftRight),
    )
}
