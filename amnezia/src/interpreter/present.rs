//! Mapping the presentation commands (screen effects, pictures, game over) to
//! the messages the presentation plugins consume. Kept beside the main loop but
//! separate so the opcode→message translation — including a picture's optional
//! wait and its direct-or-variable position — unit-tests without Bevy systems.

use super::opcodes::*;
use crate::picture::PictureCommand;
use crate::screenfx::ScreenEffect;
use crate::state::Variables;
use amnezia_data::EventCommand;

/// What a presentation command asks the interpreter to do: emit a screen effect
/// or picture command (optionally waiting `secs` for it to finish), or open the
/// Game Over screen.
pub(super) enum Present {
    Transition { kind: i32, erase: bool },
    Screen(ScreenEffect, Option<f32>),
    Picture(PictureCommand, Option<f32>),
    GameOver,
}

/// Translate presentation commands; transitions retain their type for the scene
/// controller, while tint/flash/shake/move use the command's optional wait flag.
pub(super) fn parse_present(command: &EventCommand, variables: &Variables) -> Option<Present> {
    let params = command.params.as_slice();
    let id = || params.first().copied().unwrap_or(0) as u32;
    match command.code {
        ERASE_SCREEN | SHOW_SCREEN => Some(Present::Transition {
            kind: params.first().copied().unwrap_or(0),
            erase: command.code == ERASE_SCREEN,
        }),
        TINT_SCREEN => Some(Present::Screen(
            ScreenEffect::tint(params),
            wait_secs(params, 5, 4),
        )),
        FLASH_SCREEN => Some(Present::Screen(
            ScreenEffect::flash(params),
            wait_secs(params, 5, 4),
        )),
        SHAKE_SCREEN => Some(Present::Screen(
            ScreenEffect::shake(params),
            wait_secs(params, 3, 2),
        )),
        SHOW_PICTURE => {
            let (x, y) = picture_xy(params, variables);
            Some(Present::Picture(
                PictureCommand::show(id(), &command.string, x, y, params),
                None,
            ))
        }
        MOVE_PICTURE => {
            let (x, y) = picture_xy(params, variables);
            Some(Present::Picture(
                PictureCommand::move_to(id(), x, y, params),
                wait_secs(params, 15, 14),
            ))
        }
        ERASE_PICTURE => Some(Present::Picture(PictureCommand::erase(id()), None)),
        GAME_OVER => Some(Present::GameOver),
        _ => None,
    }
}

/// The wait duration in seconds when `params[wait_index]` is set, else `None`.
/// The duration at `dur_index` is in tenths of a second.
fn wait_secs(params: &[i32], wait_index: usize, dur_index: usize) -> Option<f32> {
    let waits = params.get(wait_index).copied().unwrap_or(0) != 0;
    waits.then(|| params.get(dur_index).copied().unwrap_or(0) as f32 / 10.0)
}

/// A picture's screen coordinates: `params[2]`/`params[3]` directly, or the
/// values of those variables when `params[1]` selects the variable method.
fn picture_xy(params: &[i32], variables: &Variables) -> (f32, f32) {
    let a = params.get(2).copied().unwrap_or(0);
    let b = params.get(3).copied().unwrap_or(0);
    if params.get(1).copied().unwrap_or(0) == 1 {
        (
            variables.get(a as u32) as f32,
            variables.get(b as u32) as f32,
        )
    } else {
        (a as f32, b as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(code: u32, string: &str, params: Vec<i32>) -> EventCommand {
        EventCommand {
            code,
            indent: 0,
            string: string.to_string(),
            params,
        }
    }

    #[test]
    fn erase_and_show_preserve_the_transition_type_and_direction() {
        let vars = Variables::default();
        for kind in [-1, 0, 16, 17, 19, 20] {
            assert!(
                matches!(parse_present(&command(ERASE_SCREEN, "", vec![kind]), &vars), Some(Present::Transition { kind: actual, erase: true }) if actual == kind)
            );
            assert!(
                matches!(parse_present(&command(SHOW_SCREEN, "", vec![kind]), &vars), Some(Present::Transition { kind: actual, erase: false }) if actual == kind)
            );
        }
    }

    #[test]
    fn tint_waits_only_when_its_flag_is_set() {
        let vars = Variables::default();
        let waiting = parse_present(&command(TINT_SCREEN, "", vec![70, 70, 70, 70, 5, 1]), &vars);
        assert!(matches!(waiting, Some(Present::Screen(_, Some(s))) if (s - 0.5).abs() < 1e-6));
        let no_wait = parse_present(&command(TINT_SCREEN, "", vec![70, 70, 70, 70, 5, 0]), &vars);
        assert!(matches!(no_wait, Some(Present::Screen(_, None))));
    }

    #[test]
    fn show_picture_uses_direct_coordinates() {
        let vars = Variables::default();
        let params = vec![2, 0, 160, 120, 0, 100, 25, 1, 100, 100, 100, 100, 0, 60];
        match parse_present(&command(SHOW_PICTURE, "Map", params), &vars) {
            Some(Present::Picture(PictureCommand::Show { id, x, y, .. }, None)) => {
                assert_eq!((id, x, y), (2, 160.0, 120.0));
            }
            _ => panic!("expected a Show picture"),
        }
    }

    #[test]
    fn show_picture_resolves_variable_coordinates() {
        let mut vars = Variables::default();
        vars.set(12, 48);
        vars.set(13, 96);
        let params = vec![1, 1, 12, 13, 0, 100, 100, 1, 100, 100, 100, 0, 0, 60];
        match parse_present(&command(SHOW_PICTURE, "Intro1", params), &vars) {
            Some(Present::Picture(PictureCommand::Show { x, y, .. }, _)) => {
                assert_eq!((x, y), (48.0, 96.0));
            }
            _ => panic!("expected a Show picture"),
        }
    }

    #[test]
    fn move_picture_waits_from_param_15() {
        let vars = Variables::default();
        let params = vec![
            1,
            1,
            12,
            13,
            0,
            100,
            0,
            0,
            100,
            100,
            100,
            0,
            0,
            -2147483640,
            10,
            1,
        ];
        assert!(matches!(
            parse_present(&command(MOVE_PICTURE, "", params), &vars),
            Some(Present::Picture(PictureCommand::Move { .. }, Some(s))) if (s - 1.0).abs() < 1e-6
        ));
    }

    #[test]
    fn game_over_maps_through() {
        let vars = Variables::default();
        assert!(matches!(
            parse_present(&command(GAME_OVER, "", vec![]), &vars),
            Some(Present::GameOver)
        ));
    }

    #[test]
    fn non_presentation_command_is_none() {
        let vars = Variables::default();
        assert!(parse_present(&command(WAIT, "", vec![10]), &vars).is_none());
    }
}
