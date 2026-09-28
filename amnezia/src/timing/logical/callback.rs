use bevy::prelude::*;

pub(super) fn suspended(world: &World) -> bool {
    world
        .get_resource::<crate::transitions::Transition>()
        .is_some_and(|transition| transition.busy())
}

pub(super) fn ready(world: &World) -> bool {
    if suspended(world) {
        return false;
    }
    let inn = world.get_resource::<crate::shop::inn::State>();
    inn.is_some_and(|inn| inn.callback_ready())
        || (!inn.is_some_and(|inn| inn.resting())
            && world
                .get_resource::<crate::teleport::Fade>()
                .is_some_and(|fade| fade.busy()))
        || world
            .get_resource::<crate::interpreter::continuation::Continuation>()
            .is_some_and(|continuation| continuation.ready(inn.is_some_and(|inn| inn.resting())))
}

pub(super) fn scene(world: &World) -> (u64, bool) {
    (
        world
            .get_resource::<crate::interpreter::scenes::Requests>()
            .map_or(0, |requests| requests.committed()),
        world
            .get_resource::<crate::title::TitleActive>()
            .is_some_and(|title| title.0),
    )
}
