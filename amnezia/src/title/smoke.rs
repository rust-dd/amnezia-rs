pub(crate) use super::view::smoke::{snapshot, verify_finished, verify_load_finished};
use super::*;

mod new_game;
pub(crate) use new_game::{observe as new_game_frame, verify_finished as verify_new_game_finished};

pub(crate) fn ready(world: &World) -> bool {
    world.resource::<TitleActive>().0 && world.resource::<TitleState>().stage == flow::Stage::Ready
}

pub(crate) fn assert_continue(world: &World) {
    assert!(ready(world));
    assert_eq!(world.resource::<TitleState>().cursor, CONTINUE);
}

pub(crate) fn select_new_game(world: &mut World) {
    world.resource_mut::<TitleState>().cursor = NEW_GAME;
    new_game::begin(world);
}

pub(crate) fn return_input(frame: u32) -> Option<KeyCode> {
    match frame {
        305 | 414 => Some(KeyCode::Escape),
        320 | 330 | 340 | 350 => Some(KeyCode::ArrowDown),
        448 => Some(KeyCode::ArrowUp),
        351 | 380 | 397 | 432 | 462 => Some(KeyCode::Enter),
        _ => None,
    }
}

pub(crate) fn return_scene(world: &mut World, frame: u32) -> Option<&'static str> {
    if let Some(label) = super::view::smoke::opening_label(world, frame) {
        return Some(label);
    }
    match frame {
        368 => {
            assert!(world.resource::<crate::menu::MenuOpen>().0);
            Some("title-return-menu")
        }
        379 => Some("end-game-no-blink"),
        396 | 430 => {
            crate::menu::end_smoke::assert_cancelled(world);
            None
        }
        460 => Some("end-game-yes"),
        463 => {
            assert!(world.resource::<TitleActive>().0);
            assert!(!world.resource::<crate::menu::MenuOpen>().0);
            assert!(world.resource::<crate::transitions::Transition>().busy());
            Some("title-return-fade")
        }
        552 => {
            assert!(ready(world));
            Some("title-return-ready")
        }
        _ => None,
    }
}
