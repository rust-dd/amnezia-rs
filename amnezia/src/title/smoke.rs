use super::*;

pub(crate) fn ready(world: &World) -> bool {
    world.resource::<TitleActive>().0 && world.resource::<TitleState>().stage == flow::Stage::Ready
}

pub(crate) fn select_new_game(world: &mut World) {
    world.resource_mut::<TitleState>().cursor = NEW_GAME;
}

pub(crate) fn return_input(frame: u32) -> Option<KeyCode> {
    match frame {
        310 => Some(KeyCode::Escape),
        320 | 330 | 340 | 350 => Some(KeyCode::ArrowDown),
        366 => Some(KeyCode::ArrowUp),
        360 | 370 => Some(KeyCode::Enter),
        _ => None,
    }
}

pub(crate) fn return_scene(world: &World, frame: u32) -> Option<&'static str> {
    match frame {
        365 => {
            assert!(world.resource::<crate::menu::MenuOpen>().0);
            Some("title-return-menu")
        }
        371 => {
            assert!(world.resource::<TitleActive>().0);
            assert!(!world.resource::<crate::menu::MenuOpen>().0);
            assert!(world.resource::<crate::transitions::Transition>().busy());
            Some("title-return-fade")
        }
        460 => {
            assert!(ready(world));
            Some("title-return-ready")
        }
        _ => None,
    }
}
