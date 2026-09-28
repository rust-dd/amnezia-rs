pub(crate) use super::view::smoke::{snapshot, verify_load_finished};
use super::*;

mod new_game;
pub(in crate::title) mod repetition;
mod returning;
pub(crate) use new_game::{observe as new_game_frame, verify_finished as verify_new_game_finished};
pub(crate) fn held_input(world: &mut World, frame: u32) -> bool {
    returning::repetition_frame(world, frame)
        .is_some_and(|frame| repetition::held_input(world, frame))
}

pub(crate) fn configure(app: &mut App) {
    new_game::configure(app);
}

pub(crate) fn verify_finished(world: &World) {
    super::view::smoke::verify_finished(world);
    repetition::verify_finished(world);
    returning::verify_finished(world);
}

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

pub(crate) fn return_input(world: &mut World, frame: u32) -> Option<KeyCode> {
    returning::input(world, frame)
}

pub(crate) fn return_scene(world: &mut World, frame: u32) -> Option<&'static str> {
    if let Some(label) = super::view::smoke::opening_label(world, frame) {
        return Some(label);
    }
    returning::drive(world, frame)
}
