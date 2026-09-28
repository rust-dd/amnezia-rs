use super::frame::MAX_STEPS_PER_FRAME;
use super::parallel::ParallelSource;
use bevy::prelude::*;

pub(crate) mod smoke;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AsyncOp {
    Inn,
    Transition {
        kind: i32,
        erase: bool,
        center: IVec2,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Owner {
    Parallel(ParallelSource),
    Foreground,
    Message,
}

impl Owner {
    pub(super) fn interpreter(source: Option<ParallelSource>) -> Self {
        source.map_or(Self::Foreground, Self::Parallel)
    }
}

struct Pending {
    owner: Owner,
    remaining: usize,
    wait: WaitFor,
}

#[derive(Clone, Copy)]
enum WaitFor {
    Transition,
    Inn,
}

#[derive(Default, Clone, Copy)]
enum Pass {
    #[default]
    Fresh,
    Waiting,
    Resume(Owner),
}

/// An asynchronous operation resumes the interrupted map visit, not a new frame.
#[derive(Resource, Default)]
pub(crate) struct Continuation {
    pending: Option<Pending>,
    pass: Pass,
}

impl Continuation {
    pub(crate) fn ready(&self, inn: bool) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| match pending.wait {
                WaitFor::Transition => true,
                WaitFor::Inn => !inn,
            })
    }

    pub(super) fn resume_owner(&self) -> Option<Owner> {
        match self.pass {
            Pass::Resume(owner) => Some(owner),
            _ => None,
        }
    }

    pub(crate) fn waiting(&self) -> bool {
        matches!(self.pass, Pass::Waiting)
    }

    pub(crate) fn characters_paused(&self, frame_waiting: bool) -> bool {
        match self.pass {
            Pass::Fresh => frame_waiting,
            Pass::Waiting | Pass::Resume(Owner::Foreground | Owner::Message) => true,
            Pass::Resume(Owner::Parallel(_)) => false,
        }
    }

    pub(crate) fn effects_paused(&self, frame_waiting: bool) -> bool {
        match self.pass {
            Pass::Fresh => frame_waiting,
            Pass::Waiting | Pass::Resume(Owner::Foreground) => true,
            Pass::Resume(Owner::Parallel(_) | Owner::Message) => false,
        }
    }

    pub(crate) fn tail_paused(&self, frame_waiting: bool) -> bool {
        match self.pass {
            Pass::Fresh => frame_waiting,
            Pass::Waiting => true,
            Pass::Resume(_) => false,
        }
    }
}

pub(super) fn begin(world: &mut World) {
    let callback = world
        .get_resource::<crate::timing::logical::Step>()
        .is_some_and(|step| step.callback);
    let can_resume = callback
        || !world.contains_resource::<crate::timing::logical::Step>()
        || world.contains_resource::<super::destination::Visit>();
    let transition = world.resource::<crate::transitions::Transition>().busy();
    let inn = world
        .get_resource::<crate::shop::inn::State>()
        .is_some_and(|inn| inn.resting());
    let empty = if callback && !world.contains_resource::<super::destination::Visit>() {
        Pass::Waiting
    } else {
        Pass::Fresh
    };
    let mut state = world.resource_mut::<Continuation>();
    state.pass = state.pending.as_ref().map_or(empty, |pending| {
        if !can_resume
            || match pending.wait {
                WaitFor::Transition => transition,
                WaitFor::Inn => inn,
            }
        {
            Pass::Waiting
        } else {
            Pass::Resume(pending.owner)
        }
    });
    if matches!(state.pass, Pass::Resume(Owner::Message)) {
        state.pending = None;
    }
}

pub(super) fn budget(world: &mut World, owner: Owner) -> usize {
    let mut state = world.resource_mut::<Continuation>();
    if state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.owner == owner)
        && !state.waiting()
    {
        state.pending.take().unwrap().remaining
    } else {
        MAX_STEPS_PER_FRAME
    }
}

pub(super) fn complete_parallel(world: &mut World) {
    let mut state = world.resource_mut::<Continuation>();
    if matches!(state.pass, Pass::Resume(Owner::Parallel(_))) {
        state.pending = None;
    }
}

pub(super) fn finish_destination(world: &mut World) {
    world.resource_mut::<Continuation>().pass = Pass::Fresh;
}

pub(super) fn suspend(world: &mut World, owner: Owner, remaining: usize, op: AsyncOp) -> bool {
    let (wait, busy) = match op {
        AsyncOp::Transition { .. } => {
            world.run_system_cached_with(apply, op).unwrap();
            (
                WaitFor::Transition,
                world.resource::<crate::transitions::Transition>().busy(),
            )
        }
        AsyncOp::Inn => {
            crate::shop::inn::start_free(world);
            (
                WaitFor::Inn,
                world.resource::<crate::shop::inn::State>().resting(),
            )
        }
    };
    if !busy {
        return false;
    }
    let mut state = world.resource_mut::<Continuation>();
    state.pending = Some(Pending {
        owner,
        remaining,
        wait,
    });
    state.pass = Pass::Waiting;
    true
}

pub(crate) fn suspend_message(world: &mut World) {
    let mut state = world.resource_mut::<Continuation>();
    state.pending = Some(Pending {
        owner: Owner::Message,
        remaining: 0,
        wait: WaitFor::Inn,
    });
    state.pass = Pass::Waiting;
}

fn apply(
    In(op): In<AsyncOp>,
    mut transitions: crate::transitions::TransitionIo,
    destination: Option<Res<super::destination::Visit>>,
) {
    match op {
        AsyncOp::Inn => unreachable!(),
        AsyncOp::Transition {
            kind,
            erase,
            center,
        } => {
            let was_erased = transitions.state.event_erased;
            if transitions.event(kind, erase, center) && erase && destination.is_some() {
                transitions.state.event_erased = was_erased;
            }
        }
    }
}
