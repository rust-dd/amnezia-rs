use super::{continuation, driver, parallel};
use bevy::prelude::*;

#[derive(Resource)]
pub(super) struct Enabled;

#[derive(Resource)]
pub(crate) struct Visit;

pub(crate) fn run(world: &mut World, foreground: bool) -> bool {
    if !world.contains_resource::<Enabled>() {
        return true;
    }
    world.insert_resource(Visit);
    if foreground {
        continuation::begin(world);
        driver::foreground(world);
    } else {
        parallel::run_parallel(world);
    }
    world.remove_resource::<Visit>();
    let finished = !world.resource::<continuation::Continuation>().waiting();
    if finished {
        continuation::finish_destination(world);
    }
    finished
}
