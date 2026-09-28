use super::{Fade, PendingTeleport, Phase, rebuild, scene};
use crate::transitions::{Kind, Transition, TransitionIo};
use bevy::prelude::*;

pub(super) fn drive(world: &mut World) {
    if world.resource::<Transition>().busy() {
        return;
    }
    if world.resource::<Fade>().phase == Phase::Idle {
        if !world.resource::<PendingTeleport>().reloading() {
            return;
        }
        world.run_system_cached(begin).unwrap();
    }
    while !world.resource::<Transition>().busy() {
        match world.resource::<Fade>().phase {
            Phase::Idle => return,
            Phase::Out => arrive(world),
            Phase::Parallel => {
                if !preupdate(world, false) {
                    return;
                }
                if recurse(world) {
                    continue;
                }
                world.run_system_cached(show).unwrap();
                world.resource_mut::<Fade>().phase = Phase::In;
            }
            Phase::In => world.resource_mut::<Fade>().phase = Phase::Foreground,
            Phase::Foreground => {
                if world.resource::<Fade>().foreground {
                    if !preupdate(world, true) {
                        return;
                    }
                    if recurse(world) {
                        continue;
                    }
                }
                world.resource_mut::<Fade>().phase = Phase::Idle;
                crate::interpreter::scenes::after_transfer(world);
                return;
            }
        }
    }
}

fn preupdate(world: &mut World, foreground: bool) -> bool {
    let finished = crate::interpreter::destination::run(world, foreground);
    crate::dialogue::presentation::flush(world);
    if world
        .get_resource::<crate::title::TitleActive>()
        .is_some_and(|title| title.0)
    {
        *world.resource_mut::<Fade>() = default();
        *world.resource_mut::<PendingTeleport>() = default();
        crate::interpreter::scenes::after_transfer(world);
        return false;
    }
    finished
}

fn arrive(world: &mut World) {
    let (target, reload) = {
        let mut fade = world.resource_mut::<Fade>();
        (fade.target.take(), std::mem::take(&mut fade.reload))
    };
    if let Some(target) = target {
        crate::picture::apply_pending(world);
        world
            .run_system_cached_with(scene::normal, (target, reload))
            .unwrap();
        rebuild::flush(world);
        if reload {
            crate::world::update::begin(world);
        }
        crate::player::relocate_camera(world);
    }
    world.resource_mut::<Fade>().phase = Phase::Parallel;
}

fn recurse(world: &mut World) -> bool {
    let Some(target) = world.resource_mut::<PendingTeleport>().0.take() else {
        return false;
    };
    let mut pending = world.resource_mut::<PendingTeleport>();
    pending.1 = false;
    pending.3 = false;
    let mut fade = world.resource_mut::<Fade>();
    fade.target = Some(target);
    fade.phase = Phase::Out;
    true
}

pub(super) fn begin_pending(world: &mut World) {
    if world.resource::<PendingTeleport>().0.is_none()
        || world
            .run_system_cached(|scene: crate::world::ScenePause| scene.tail_paused())
            .unwrap()
    {
        return;
    }
    world.run_system_cached(begin).unwrap();
    drive(world);
}

fn begin(
    mut transition: TransitionIo,
    mut pending: ResMut<PendingTeleport>,
    mut fade: ResMut<Fade>,
) {
    let Some(target) = pending.0.take() else {
        return;
    };
    fade.target = Some(target);
    fade.reload = std::mem::take(&mut pending.1);
    fade.default_show = fade.reload;
    fade.foreground = std::mem::take(&mut pending.3);
    let kind = if fade.reload {
        Kind::Fade
    } else {
        transition.kind(0)
    };
    if !transition.state.erased() {
        transition
            .state
            .start(kind, true, transition.frames.frame, IVec2::new(160, 120));
    }
    fade.phase = Phase::Out;
}

fn show(mut transition: TransitionIo, fade: Res<Fade>) {
    let kind = if fade.default_show && transition.state.erased() {
        Kind::Fade
    } else if !fade.default_show && !transition.state.event_erased {
        transition.kind(1)
    } else {
        return;
    };
    let now = transition.frames.frame;
    transition
        .state
        .start(kind, false, now, IVec2::new(160, 120));
}
