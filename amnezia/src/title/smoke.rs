pub(crate) use super::view::smoke::{snapshot, verify_finished, verify_load_finished};
use super::*;

pub(crate) fn ready(world: &World) -> bool {
    world.resource::<TitleActive>().0 && world.resource::<TitleState>().stage == flow::Stage::Ready
}

pub(crate) fn assert_continue(world: &World) {
    assert!(ready(world));
    assert_eq!(world.resource::<TitleState>().cursor, CONTINUE);
}

pub(crate) fn select_new_game(world: &mut World) {
    world.resource_mut::<TitleState>().cursor = NEW_GAME;
}

pub(crate) fn return_input(frame: u32) -> Option<KeyCode> {
    match frame {
        310 | 395 => Some(KeyCode::Escape),
        320 | 330 | 340 | 350 => Some(KeyCode::ArrowDown),
        410 => Some(KeyCode::ArrowUp),
        360 | 380 | 390 | 400 | 420 => Some(KeyCode::Enter),
        _ => None,
    }
}

pub(crate) fn return_scene(world: &mut World, frame: u32) -> Option<&'static str> {
    if let Some(label) = super::view::smoke::opening_label(world, frame) {
        return Some(label);
    }
    match frame {
        365 => {
            assert!(world.resource::<crate::menu::MenuOpen>().0);
            Some("title-return-menu")
        }
        372 => Some("end-game-no-blink"),
        385 | 397 => {
            crate::menu::end_smoke::assert_cancelled(world);
            None
        }
        415 => Some("end-game-yes"),
        421 => {
            assert!(world.resource::<TitleActive>().0);
            assert!(!world.resource::<crate::menu::MenuOpen>().0);
            assert!(world.resource::<crate::transitions::Transition>().busy());
            Some("title-return-fade")
        }
        510 => {
            assert!(ready(world));
            Some("title-return-ready")
        }
        _ => None,
    }
}
